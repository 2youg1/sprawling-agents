// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use super::{ClaimDesk, Pinner};
use crate::claim_effect::ClaimEffect;
use kernel::spine::check_roadmap_shape;
use kernel::{
    AxCode, AxError, CostTier, Effect, NewChild, NodeId, Payload, PlanTree, RenderIntent,
    RoadmapShape, StopCause, Temporal, Tool, ToolCall, ToolDef, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

pub struct ClaimTool {
    meta: ToolMeta,
    desk: Arc<Mutex<ClaimDesk>>,
    pinner: Option<Pinner>,
}

impl ClaimTool {
    pub fn definition() -> Result<ToolDef, AxError> {
        let mut properties = Map::new();
        for (field, kind, description) in [
            (
                "action",
                "string",
                "`list` | `claim` | `finish` | `block` | `release` | `split` | `add` (Mayor)",
            ),
            ("node", "string", "dotted index, such as `2.3`"),
            ("evidence", "string", "a path or locator"),
            ("reason", "string", "one line: why"),
            (
                "parts",
                "array",
                "text, or `{item, weight, needs}`; needs: existing row IDs",
            ),
        ] {
            let mut spec = Map::new();
            spec.insert("type".to_owned(), Value::String(kind.to_owned()));
            spec.insert(
                "description".to_owned(),
                Value::String(description.to_owned()),
            );
            properties.insert(field.to_owned(), Value::Object(spec));
        }
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("action".to_owned())]),
        );
        Ok(ToolDef {
            name: ToolName::parse("plan")?,
            description: "Take a node of this building's plan before unassigned work. \
                          Must this be expanded?"
                .to_owned(),
            input_schema: Payload::new(params)?,
        })
    }

    pub fn new(desk: Arc<Mutex<ClaimDesk>>) -> Result<ClaimTool, AxError> {
        let defined = Self::definition()?;
        let room = desk
            .lock()
            .map_err(|_| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "reach the plan desk",
                    "the desk was left locked by a thread that died",
                )
                .with_recovery("restart this city")
            })?
            .room
            .clone();
        Ok(ClaimTool {
            meta: ToolMeta {
                name: defined.name,
                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Write { domain: room },
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
            desk,
            pinner: None,
        })
    }

    #[must_use]
    pub fn pinning(self, pinner: Pinner) -> ClaimTool {
        ClaimTool {
            pinner: Some(pinner),
            ..self
        }
    }
}

impl Tool for ClaimTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read the plan",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let args = call.args.as_map();
        let mut desk = self.desk.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "reach the plan desk",
                "the desk was left locked by a thread that died",
            )
            .with_recovery("restart this city")
        })?;
        let action = args.get("action").and_then(Value::as_str).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a plan action",
                "missing string argument `action`",
            )
            .with_recovery(ACTIONS)
        })?;
        let result = match action {
            "list" => desk.list()?,
            "claim" => desk.claim(&node_of(args)?)?,
            "finish" => {
                let evidence = args
                    .get("evidence")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AxError::failure(
                            AxCode::EvidenceMissing,
                            "finish a plan node",
                            "missing string argument `evidence`",
                        )
                        .with_recovery(
                            "pass the path of the file that shows the work, or a locator: \
                             `cas:<hash>` or `file:<path>@<oid>`",
                        )
                    })?;
                let evidence = Pinner::evidence(self.pinner.as_ref(), evidence)?;
                desk.finish(&node_of(args)?, evidence)?
            }
            "block" => {
                let note = reason_of(args, "block")?;
                desk.put_down(&node_of(args)?, StopCause::Blocked { note })?
            }
            "release" => {
                let note = reason_of(args, "release")?;
                desk.put_down(&node_of(args)?, StopCause::HandedBack { note })?
            }
            "split" => desk.split(&node_of(args)?, &parts_of(args)?)?,
            "add" => desk.add(&parts_of(args)?)?,
            other => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "read a plan action",
                    other.to_owned(),
                )
                .with_recovery(ACTIONS));
            }
        };
        Ok(ToolOutcome {
            result,
            attachments: Vec::new(),
        })
    }
}

impl ClaimDesk {
    fn add(&mut self, rows: &[NewChild]) -> Result<Payload, AxError> {
        let refuse = |subject: String, recovery: String| {
            AxError::failure(AxCode::InvalidArgs, "add a plan row", subject).with_recovery(recovery)
        };
        if self.room.as_str() != kernel::consts_policy::HALL_MAYOR {
            return Err(refuse(
                format!(
                    "the plan's root belongs to the building's Mayor, and this run is {}",
                    self.room
                ),
                format!(
                    "claim a row and split it to add work under it, or signal {} to add a \
                     top-level row",
                    kernel::consts_policy::HALL_MAYOR
                ),
            ));
        }
        if rows.is_empty() {
            return Err(refuse(
                "no rows given".to_owned(),
                "pass `parts` as the rows to add, each `{item, weight}` or a plain string"
                    .to_owned(),
            ));
        }
        let tree = self.tree()?;
        if let Some(missing) = rows
            .iter()
            .flat_map(|row| &row.needs)
            .find(|need| tree.get(need).is_none())
        {
            return Err(refuse(
                format!("no existing row numbered {missing}"),
                "name an existing prerequisite; add it before adding work that needs it".to_owned(),
            ));
        }
        let last = tree
            .nodes()
            .filter(|node| node.row.id.parent().is_none())
            .map(|node| node.row.id.ordinal())
            .max()
            .unwrap_or(0);
        let mut grown = self.text.clone();
        let mut added = Vec::with_capacity(rows.len());
        let mut effects = Vec::with_capacity(rows.len());
        for (offset, row) in (1u32..).zip(rows) {
            let ordinal = last.checked_add(offset).ok_or_else(|| {
                refuse(
                    "the plan has no top-level index left".to_owned(),
                    "split an existing row instead".to_owned(),
                )
            })?;
            let id = NodeId::parse(&ordinal.to_string())?;
            let effect = ClaimEffect::Added {
                id: id.clone(),
                child: NewChild {
                    item: row.item.trim().to_owned(),
                    weight: row.weight,
                    needs: row.needs.clone(),
                },
            };
            grown = effect.apply(&grown)?;
            added.push(Value::String(id.to_string()));
            effects.push(effect);
        }
        let RoadmapShape::WellFormed { rows: parsed } = check_roadmap_shape(&grown) else {
            return Err(refuse(
                "the rows would leave a table that does not parse".to_owned(),
                "shorten the items; each must fit one table row".to_owned(),
            ));
        };
        PlanTree::build(parsed)?;
        self.text = grown;
        self.changed = true;
        self.effects.extend(effects);
        let mut result = Map::new();
        result.insert("nodes".to_owned(), Value::Array(added));
        Payload::new(result)
    }
}

pub(super) const ACTIONS: &str =
    "use `list`, `claim`, `finish`, `block`, `release`, `split` or `add`";

fn node_of(args: &Map<String, Value>) -> Result<NodeId, AxError> {
    let raw = args.get("node").and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a plan node index",
            "missing string argument `node`",
        )
        .with_recovery("pass `node` as the dotted index in the plan's first column")
    })?;
    NodeId::parse(raw)
}

fn reason_of(args: &Map<String, Value>, action: &str) -> Result<String, AxError> {
    let raw = args
        .get("reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read why a plan node stopped",
                format!("`{action}` without a reason"),
            )
            .with_recovery(
                "say in one line why, so the next run does not repeat what you already tried",
            )
        })?;
    Ok(raw.to_owned())
}

fn parts_of(args: &Map<String, Value>) -> Result<Vec<NewChild>, AxError> {
    let refuse = |why: &str| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read the parts of a plan change",
            why.to_owned(),
        )
        .with_recovery("pass text or `{item, weight, needs}` in `parts`; use an array of existing row IDs for `needs`")
    };
    let listed = args
        .get("parts")
        .and_then(Value::as_array)
        .ok_or_else(|| refuse("missing array argument `parts`"))?;
    let mut children = Vec::with_capacity(listed.len());
    for entry in listed {
        let child = match entry {
            Value::String(item) => NewChild {
                item: item.clone(),
                weight: 1,
                needs: Vec::new(),
            },
            Value::Object(map) => {
                let item = map
                    .get("item")
                    .and_then(Value::as_str)
                    .ok_or_else(|| refuse("a part with no `item`"))?;
                let weight = map.get("weight").and_then(Value::as_u64).unwrap_or(1);
                let mut needs = Vec::new();
                if let Some(value) = map.get("needs") {
                    let listed = value
                        .as_array()
                        .ok_or_else(|| refuse("`needs` is not an array"))?;
                    let mut seen = BTreeSet::new();
                    for value in listed {
                        let raw = value
                            .as_str()
                            .ok_or_else(|| refuse("a `needs` entry is not a row ID string"))?;
                        let node = NodeId::parse(raw)?;
                        if !seen.insert(node.clone()) {
                            return Err(refuse("a row ID appears twice in `needs`"));
                        }
                        needs.push(node);
                    }
                }
                NewChild {
                    item: item.to_owned(),
                    weight: u32::try_from(weight).unwrap_or(u32::MAX),
                    needs,
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => {
                return Err(refuse("a part that is neither text nor an object"));
            }
        };
        children.push(child);
    }
    Ok(children)
}
