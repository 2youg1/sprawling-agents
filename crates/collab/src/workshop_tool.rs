// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face a workshop shows a model: split one creation into nodes,
//! hand each down, and judge the join.
//!
//! Three verbs, because three different things happen. `lay_out` turns a
//! list of nodes into a graph that can finish - duplicates, dangling
//! dependencies and cycles are refused at construction, so a workshop
//! that exists is one that terminates - and hands each node down in
//! schedule order. `question` asks what the join wants answered before
//! anybody may judge it. `judge` answers it.
//!
//! **Nothing here starts a run and nothing here decides who may.** Every
//! node goes through [`DelegateDesk`], so the one-level rule and the
//! person's permission are the same two doors a single `delegate` call
//! passes, reached by the same code. A workshop is fan-out over
//! delegation, not a second way to spawn.
//!
//! A node's contract is its `JOB.md`. The contract is the whole of what
//! its agent is answerable for, and writing a summary of it into the job
//! file would put the authority in two places.

use std::collections::BTreeSet;

use kernel::{
    Address, AxCode, AxError, BudgetCap, CostTier, DelegateKind, Effect, Payload, RenderIntent,
    Temporal, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::delegate_tool::{DelegateDesk, Delegated};
use crate::fanin::{Artifact, FanIn, Joined, PrivateQuestion};
use crate::workshop::{NodeContract, NodeId, Workshop};

/// One run's workshop: the graph it laid out, and what has come back to
/// the room it works in.
#[derive(Debug)]
pub struct WorkshopDesk {
    who: String,
    laid_out: Option<Workshop>,
    joined: FanIn,
}

impl WorkshopDesk {
    /// `joined` is what the city already holds for this room, folded
    /// from the handbacks its earlier runs received. A join outlives one
    /// run because the nodes do: a child starts after its parent froze.
    #[must_use]
    pub fn new(who: String, joined: FanIn) -> WorkshopDesk {
        WorkshopDesk {
            who,
            laid_out: None,
            joined,
        }
    }

    /// Accepts a graph and hands every node down, in the order the graph
    /// itself decides.
    ///
    /// # Errors
    /// Propagates the graph's own refusals - a duplicate id, a
    /// dependency on a node that is not there, a cycle - and the
    /// delegate desk's, which are the depth and building rules. Refuses
    /// a second lay-out in one run: two graphs in one session is two
    /// answers to what this run is building.
    pub fn lay_out(
        &mut self,
        contracts: Vec<NodeContract>,
        delegates: &mut DelegateDesk,
    ) -> Result<Vec<NodeId>, AxError> {
        if self.laid_out.is_some() {
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
        let workshop = Workshop::new(contracts)?;
        let schedule = workshop.schedule();
        // Asked before anything is kept: a graph half handed down is a
        // graph whose remaining nodes nobody will start.
        for id in &schedule {
            let contract = workshop.contract(id).ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "lay out a workshop",
                    format!("{} is scheduled and has no contract", id.as_str()),
                )
            })?;
            delegates.ask(Delegated {
                room: contract.write_domain().clone(),
                task: contract.job_text(),
                goal: contract.done_check().to_owned(),
                kind: DelegateKind::Ephemeral,
            })?;
        }
        self.laid_out = Some(workshop);
        Ok(schedule)
    }

    /// What the join asks before it will take a verdict.
    ///
    /// # Errors
    /// Refuses when nothing has come back: there is nothing to have
    /// read.
    pub fn question(&self) -> Result<PrivateQuestion, AxError> {
        self.joined.question()
    }

    /// The verdict, admitted only with the answer.
    ///
    /// # Errors
    /// Propagates the join's refusal of an answer that could have been
    /// given without opening anything.
    pub fn judge(&self, answer: &str) -> Result<Joined, AxError> {
        self.joined.decide(answer)
    }

    /// Who this desk speaks for.
    #[must_use]
    pub fn who(&self) -> &str {
        &self.who
    }

    /// Adds one verified result to the join.
    pub fn accept(&mut self, artifact: Artifact) {
        self.joined.accept(artifact);
    }
}

/// The tool itself.
pub struct WorkshopTool {
    desk: std::rc::Rc<std::cell::RefCell<WorkshopDesk>>,
    delegates: std::rc::Rc<std::cell::RefCell<DelegateDesk>>,
    meta: ToolMeta,
}

/// The three things this tool does. Exhaustive: an unknown verb is
/// refused rather than rounded to the harmless one, because the harmless
/// one here would silently drop a graph somebody meant to run.
enum Op {
    LayOut,
    Question,
    Judge,
}

impl Op {
    fn parse(raw: &str) -> Result<Op, AxError> {
        match raw {
            "lay_out" => Ok(Op::LayOut),
            "question" => Ok(Op::Question),
            "judge" => Ok(Op::Judge),
            other => Err(AxError::failure(
                AxCode::InvalidArgs,
                "run a workshop",
                format!("no such operation: {other}"),
            )
            .with_recovery("lay_out to split the work, question then judge to close the join")),
        }
    }
}

impl WorkshopTool {
    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(
        desk: std::rc::Rc<std::cell::RefCell<WorkshopDesk>>,
        delegates: std::rc::Rc<std::cell::RefCell<DelegateDesk>>,
    ) -> Result<WorkshopTool, AxError> {
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
        Ok(WorkshopTool {
            desk,
            delegates,
            meta: ToolMeta {
                name: ToolName::parse("workshop")?,
                disclosure: "Split one creation into nodes that run in dependency order, each \
                             in its own room, and judge what comes back. Fan-out over \
                             delegation: the person is asked once, as for `delegate`."
                    .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Spawn,
                cost_tier: CostTier::Heavy,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }
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

/// One node as the model wrote it, turned into the contract the agent
/// working there is answerable for.
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
        BudgetCap::default(),
        text(map, "stop")?,
    )
}

impl Tool for WorkshopTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "run a workshop",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            ));
        }
        let args = call.args.as_map();
        let mut desk = self.desk.try_borrow_mut().map_err(|_| {
            AxError::failure(AxCode::InvalidArgs, "run a workshop", "the desk is in use")
        })?;
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
                let mut delegates = self.delegates.try_borrow_mut().map_err(|_| {
                    AxError::failure(AxCode::InvalidArgs, "run a workshop", "the desk is in use")
                })?;
                let schedule = desk.lay_out(contracts, &mut delegates)?;
                out.insert(
                    "schedule".to_owned(),
                    Value::Array(
                        schedule
                            .iter()
                            .map(|id| Value::String(id.as_str().to_owned()))
                            .collect(),
                    ),
                );
                out.insert(
                    "starts".to_owned(),
                    Value::String("when this turn settles, in that order".to_owned()),
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
