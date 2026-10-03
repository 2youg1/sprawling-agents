// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The tool itself: the seven actions, and the arguments they are
//! spelled with.

use std::sync::{Arc, Mutex};

use super::ClaimDesk;
use crate::claim_effect::ClaimEffect;
use kernel::spine::check_roadmap_shape;
use kernel::{
    AxCode, AxError, CostTier, Effect, NewChild, NodeId, Payload, PlanTree, RenderIntent,
    RoadmapShape, StopCause, Temporal, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

/// The tool itself: a thin router onto the desk.
pub struct ClaimTool {
    meta: ToolMeta,
    desk: Arc<Mutex<ClaimDesk>>,
}

impl ClaimTool {
    /// # Errors
    /// Propagates a malformed tool name or parameter schema, neither of
    /// which can happen with the literals below.
    pub fn new(desk: Arc<Mutex<ClaimDesk>>) -> Result<ClaimTool, AxError> {
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
        let mut properties = Map::new();
        for (field, kind, description) in [
            (
                "action",
                "string",
                "`list` | `claim` | `finish` (evidence) | `block` (reason) | `release` (reason) \
                 | `split` (parts) | `add` (parts; Mayor, empty plan)",
            ),
            ("node", "string", "dotted index, such as `2.3`"),
            ("evidence", "string", "a retrievable locator"),
            ("reason", "string", "one line: why"),
            ("parts", "array", "the children, each `{item, weight}`"),
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
        Ok(ClaimTool {
            meta: ToolMeta {
                name: ToolName::parse("plan")?,
                // The question sits in the cached prefix, so it costs no
                // latency and no spend; the method stays the model's.
                disclosure: "Take a node of this building's plan before unassigned work. \
                             Must this be expanded?"
                    .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Write { domain: room },
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
            desk,
        })
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
                            "pass a retrievable locator: `cas:<hash>` or `file:<path>@<oid>`",
                        )
                    })?;
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
    /// Writes a top-level row under the plan's root, the one door into
    /// an empty plan (roadmap F2).
    ///
    /// The root's share is held by the building's Mayor (kernel
    /// `Share.lean` D25), so only `hall/mayor` adds here; a run elsewhere
    /// adds under the branch it holds, which is `split`. The new index
    /// follows the last top-level row, and the grown table is rebuilt
    /// before anything is written, as a split's is.
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
        let last = self
            .tree()?
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
                },
            };
            grown = effect.apply(&grown)?;
            added.push(Value::String(id.to_string()));
            effects.push(effect);
        }
        // Built before it is written, as a split's table is: rows that
        // would not parse or build are refused with the file untouched.
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

/// The one place the seven actions are spelled for a caller that got it
/// wrong. A second list would drift from the schema above.
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

/// The children of a split, as the model wrote them.
///
/// A bare string is accepted as well as an object: a weight nobody
/// stated is 1, and refusing the shorter form would make the common case
/// — divide this evenly — the one that needs the most typing.
fn parts_of(args: &Map<String, Value>) -> Result<Vec<NewChild>, AxError> {
    let refuse = |why: &str| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read the parts of a split",
            why.to_owned(),
        )
        .with_recovery("pass `parts` as a list of `{item, weight}`, or of plain strings")
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
            },
            Value::Object(map) => {
                let item = map
                    .get("item")
                    .and_then(Value::as_str)
                    .ok_or_else(|| refuse("a part with no `item`"))?;
                let weight = map.get("weight").and_then(Value::as_u64).unwrap_or(1);
                NewChild {
                    item: item.to_owned(),
                    weight: u32::try_from(weight).unwrap_or(u32::MAX),
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
