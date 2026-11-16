// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use kernel::{
    Address, AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall,
    ToolDef, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::delegate_tool::DelegateDesk;
use crate::fanin::{Artifact, FanIn, Joined, PrivateQuestion};
use crate::workshop::{Laid, LaidOut, NodeContract, NodeId, Underway, Workshop};

#[derive(Debug)]
pub struct WorkshopDesk {
    who: String,

    handed: BTreeSet<NodeId>,
    underway: Laid,
    joined: FanIn,
}

impl WorkshopDesk {

    #[must_use]
    pub fn new(who: String, joined: FanIn, handed: BTreeSet<NodeId>) -> WorkshopDesk {
        WorkshopDesk {
            who,
            handed,
            underway: Laid::Nothing,
            joined,
        }
    }

    pub fn lay_out(
        &mut self,
        contracts: Vec<NodeContract>,
        delegates: &mut DelegateDesk,
    ) -> Result<LaidOut, AxError> {
        if !matches!(self.underway, Laid::Nothing) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "lay out a workshop",
                "this run already laid one out",
            )
            .with_recovery(
                "one graph per session; add the work to the next session's graph, or hand a \
                 single extra piece down with `delegate`",
            ));
        }
        let done: BTreeSet<NodeId> = self
            .joined
            .artifacts()
            .map(|artifact| artifact.node().clone())
            .collect();
        let mut underway = Underway::new(
            Workshop::new(contracts)?,
            self.handed.clone(),
            delegates.beside(),
        );
        let mut handed = Vec::new();
        for work in underway.hand_next(&done)? {
            handed.push(NodeId::parse(work.room.as_str())?);
            delegates.ask(work)?;
        }
        let schedule = underway.schedule();
        let waiting = schedule
            .iter()
            .filter(|id| !done.contains(id) && !underway.handed().contains(id))
            .cloned()
            .collect();
        self.underway = Laid::Held(underway);
        Ok(LaidOut {
            schedule,
            handed,
            waiting,
        })
    }

    pub fn take_underway(&mut self) -> Option<Underway> {
        self.underway.take()
    }

    pub fn question(&self) -> Result<PrivateQuestion, AxError> {
        self.joined.question()
    }

    pub fn judge(&self, answer: &str) -> Result<Joined, AxError> {
        self.joined.decide(answer)
    }

    #[must_use]
    pub fn who(&self) -> &str {
        &self.who
    }

    pub fn accept(&mut self, artifact: Artifact) {
        self.joined.accept(artifact);
    }
}

pub struct WorkshopTool {
    desk: std::sync::Arc<std::sync::Mutex<WorkshopDesk>>,
    delegates: std::sync::Arc<std::sync::Mutex<DelegateDesk>>,
    meta: ToolMeta,
}

mod op;

use op::Op;

impl WorkshopTool {

    pub fn definition() -> Result<ToolDef, AxError> {
        let mut properties = Map::new();
        for (field, kind, description) in [
            (
                "op",
                "string",
                "`lay_out` to split this work into nodes, `question` to ask what the join wants \
                 answered, `judge` to answer it",
            ),
            (
                "nodes",
                "array",
                "for lay_out: each node is {room, goal, done_check, stop, depends_on?}. `room` \
                 is where that node works and is also its id; `depends_on` lists the rooms it \
                 waits for",
            ),
            (
                "answer",
                "string",
                "for judge: the answer to what `question` asked",
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
            Value::Array(vec![Value::String("op".to_owned())]),
        );
        Ok(ToolDef {
            name: ToolName::parse("workshop")?,
            description: "Split one creation into nodes that run in dependency order, each \
                          in its own room, and judge what comes back. Fan-out over \
                          delegation, and bounded by the same rule as `delegate`: a \
                          node hands work down no further."
                .to_owned(),
            input_schema: Payload::new(params)?,
        })
    }

    pub fn new(
        desk: std::sync::Arc<std::sync::Mutex<WorkshopDesk>>,
        delegates: std::sync::Arc<std::sync::Mutex<DelegateDesk>>,
    ) -> Result<WorkshopTool, AxError> {
        let defined = Self::definition()?;
        Ok(WorkshopTool {
            desk,
            delegates,
            meta: ToolMeta {
                name: defined.name,
                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Spawn,
                cost_tier: CostTier::Heavy,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }
}

fn poisoned(which: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "run a workshop",
        format!("{which} was left locked by a thread that died"),
    )
    .with_recovery(format!(
        "end this run and resume it: {which} cannot be reached again inside a process \
         where a thread died holding it"
    ))
}

fn text(map: &Map<String, Value>, key: &str) -> Result<String, AxError> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "run a workshop",
                format!("missing string argument `{key}`"),
            )
            .with_recovery("every node states room, goal, done_check and stop")
        })
}

fn contract_of(node: &Value, owner: &str) -> Result<NodeContract, AxError> {
    let map = node.as_object().ok_or_else(|| {
        AxError::failure(AxCode::InvalidArgs, "run a workshop", "a node is an object")
            .with_recovery("each node is {room, goal, done_check, stop, depends_on?}")
    })?;
    let room = Address::parse(&text(map, "room")?)?;
    let mut depends_on = BTreeSet::new();
    if let Some(list) = map.get("depends_on").and_then(Value::as_array) {
        for entry in list {
            let raw = entry.as_str().ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "run a workshop",
                    "a dependency is the room of another node",
                )
                .with_recovery(
                    "write each entry of `depends_on` as the room string of another \
                     node in this same call",
                )
            })?;
            depends_on.insert(NodeId::parse(raw)?);
        }
    }
    NodeContract::new(
        NodeId::parse(room.as_str())?,
        text(map, "goal")?,
        depends_on,
        Vec::new(),
        room,
        owner.to_owned(),
        text(map, "done_check")?,
        text(map, "stop")?,
    )
}

impl Tool for WorkshopTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "run a workshop",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let args = call.args.as_map();
        let mut desk = self.desk.lock().map_err(|_| poisoned("the desk"))?;
        let mut out = Map::new();
        match Op::parse(&text(args, "op")?)? {
            Op::LayOut => {
                let nodes = args
                    .get("nodes")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        AxError::failure(
                            AxCode::InvalidArgs,
                            "run a workshop",
                            "lay_out needs a `nodes` array",
                        )
                        .with_recovery("give at least one node; one node is a `delegate` call")
                    })?
                    .clone();
                let owner = desk.who().to_owned();
                let contracts = nodes
                    .iter()
                    .map(|node| contract_of(node, &owner))
                    .collect::<Result<Vec<NodeContract>, AxError>>()?;
                let mut delegates = self
                    .delegates
                    .lock()
                    .map_err(|_| poisoned("the delegation desk"))?;
                let laid = desk.lay_out(contracts, &mut delegates)?;
                for (field, ids) in [
                    ("schedule", &laid.schedule),
                    ("handed", &laid.handed),
                    ("waiting", &laid.waiting),
                ] {
                    let names = ids.iter().map(|id| Value::String(id.as_str().to_owned()));
                    out.insert(field.to_owned(), Value::Array(names.collect()));
                }
                out.insert(
                    "starts".to_owned(),
                    Value::String(
                        "`handed` now, while this run goes on; each of `waiting` once its \
                         dependencies hand back, unless this run ends cancelled or failed"
                            .to_owned(),
                    ),
                );
            }
            Op::Question => {
                let question = desk.question()?;
                out.insert(
                    "node".to_owned(),
                    Value::String(question.node().as_str().to_owned()),
                );
                out.insert(
                    "question".to_owned(),
                    Value::String(question.prompt().to_owned()),
                );
            }
            Op::Judge => {
                let joined = desk.judge(&text(args, "answer")?)?;
                out.insert(
                    "joined".to_owned(),
                    Value::Array(
                        joined
                            .nodes()
                            .iter()
                            .map(|id| Value::String(id.as_str().to_owned()))
                            .collect(),
                    ),
                );
            }
        }
        Ok(ToolOutcome {
            result: Payload::new(out)?,
            attachments: Vec::new(),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
