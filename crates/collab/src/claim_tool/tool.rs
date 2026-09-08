// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The tool itself: the six actions, and the arguments they are
//! spelled with.

use std::sync::{Arc, Mutex};

use super::ClaimDesk;
use kernel::{
    AxCode, AxError, CostTier, Effect, NewChild, NodeId, Payload, RenderIntent, StopCause,
    Temporal, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
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
                 | `split` (parts)",
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
                disclosure: "Read this building's plan and take a node before starting work \
                             nobody assigned you."
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

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read the plan",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            ));
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
            other => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "read a plan action",
                    other.to_owned(),
                )
                .with_recovery(ACTIONS));
            }
        };
        Ok(ToolOutcome { result })
    }
}

/// The one place the six actions are spelled for a caller that got it
/// wrong. A second list would drift from the schema above.
pub(super) const ACTIONS: &str = "use `list`, `claim`, `finish`, `block`, `release` or `split`";

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
            _ => return Err(refuse("a part that is neither text nor an object")),
        };
        children.push(child);
    }
    Ok(children)
}
