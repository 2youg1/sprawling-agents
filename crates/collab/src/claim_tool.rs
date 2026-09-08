// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The plan tool: the face `Roadmap.md` shows a model.
//!
//! The plan file is the only register. A second table of who-holds-what
//! would be a second answer to the same question, and the one that
//! drifts is always the one nobody reads — while this file is read by
//! the person, counted by `kernel::PlanTree`, and edited here.
//!
//! **Six actions on one catalog line.** `list` and `claim` are how a run
//! finds work without being told what to do; `finish`, `block` and
//! `release` are the ways a held node is put down; `split` is how a run
//! that has found more work says so. They are actions of the entry that
//! already existed rather than a second tool, because the number of
//! lines a model reads every turn is a cost and the number of verbs
//! behind one line is not.
//!
//! Which transitions are legal is decided by `kernel::PlanTree` from the
//! plan's own state, not from what the caller says it is doing — and the
//! refusal names both the state the node is actually in and a node that
//! is ready, because "no" teaches a model to rephrase and try again
//! while "2.3 is being worked on, 2.4 is ready" does not.

use crate::claim_effect::ClaimEffect;
use kernel::{
    Address, AxCode, AxError, Held, Locator, NewChild, NodeId, Payload, PlanExit, PlanTree,
    RoadmapShape, RoadmapStatus, StopCause, check_roadmap_shape, insert_children,
    set_roadmap_status,
};
use serde_json::{Map, Value};

/// The run's side of the plan: the file as it stood when the run was
/// dispatched, plus what this run has changed that the ledger does not
/// know yet.
#[derive(Debug)]
pub struct ClaimDesk {
    who: String,
    room: Address,
    text: String,
    changed: bool,
    /// The node this run holds. One at a time: a run holding two nodes
    /// makes both nodes' progress unreadable, because the node is the
    /// unit of "what is being worked on now".
    ///
    /// The value itself is the plan gate: it is minted only by
    /// `PlanTree::claim` and spent only on an exit, so a run cannot put
    /// work down without saying how it went.
    held: Option<Held>,
    effects: Vec<ClaimEffect>,
}

impl ClaimDesk {
    #[must_use]
    pub fn new(who: String, room: Address, roadmap: String) -> ClaimDesk {
        ClaimDesk {
            who,
            room,
            text: roadmap,
            changed: false,
            held: None,
            effects: Vec::new(),
        }
    }

    /// What the worker has to record, drained so it cannot run twice.
    pub fn take_effects(&mut self) -> Vec<ClaimEffect> {
        std::mem::take(&mut self.effects)
    }

    /// The plan as this run left it, or `None` when the run did not
    /// touch it. Writing an unchanged file would put a modification time
    /// on a plan nobody edited.
    #[must_use]
    pub fn roadmap(&self) -> Option<&str> {
        if self.changed { Some(&self.text) } else { None }
    }

    #[must_use]
    pub fn who(&self) -> &str {
        &self.who
    }

    /// The node this run is holding, if any.
    #[must_use]
    pub fn holding(&self) -> Option<&NodeId> {
        self.held.as_ref().map(Held::id)
    }

    /// Spends a still-held node on the one exit a run that simply ended
    /// has earned.
    ///
    /// **This is where red comes from.** A run that froze while holding
    /// work did not release it and did not finish it; leaving the node
    /// `In progress` for ever would strand the branch behind it, and
    /// calling it done would be a lie. The freeze path calls this, and
    /// it is a no-op for a run that put its node down properly.
    ///
    /// # Errors
    /// Propagates the plan's refusal to record the exit.
    pub fn abandon(&mut self) -> Result<(), AxError> {
        let Some(held) = self.held.take() else {
            return Ok(());
        };
        self.record(held.stop(StopCause::FrozeWithoutEvidence))
    }

    fn tree(&self) -> Result<PlanTree, AxError> {
        match check_roadmap_shape(&self.text) {
            RoadmapShape::WellFormed { rows } => PlanTree::build(rows),
            RoadmapShape::Malformed { problems } => Err(AxError::failure(
                AxCode::InvalidArgs,
                "read the plan",
                problems.join("; "),
            )
            .with_recovery(
                "repair the six-column table in Roadmap.md, then claim a node; a plan that does \
                 not parse has no denominator",
            )),
        }
    }

    /// Writes an exit into the file and queues the record.
    fn record(&mut self, exit: PlanExit) -> Result<(), AxError> {
        let tree = self.tree()?;
        let item = tree
            .get(exit.id())
            .map_or_else(String::new, |node| node.row.item.clone());
        self.text = set_roadmap_status(&self.text, exit.id(), exit.status(), exit.evidence())?;
        self.changed = true;
        self.effects.push(ClaimEffect::PutDown {
            id: exit.id().clone(),
            item,
            exit,
        });
        Ok(())
    }

    fn list(&self) -> Result<Payload, AxError> {
        let tree = self.tree()?;
        let mut ready = Vec::new();
        let mut working = Vec::new();
        let mut red = Vec::new();
        for node in tree.nodes() {
            let mut entry = Map::new();
            entry.insert("node".to_owned(), Value::String(node.row.id.to_string()));
            entry.insert("item".to_owned(), Value::String(node.row.item.clone()));
            match node.row.status {
                RoadmapStatus::InProgress if node.is_leaf() => working.push(Value::Object(entry)),
                RoadmapStatus::Blocked | RoadmapStatus::AwaitingApproval => {
                    red.push(Value::Object(entry));
                }
                _ => {}
            }
        }
        for id in tree.ready() {
            let mut entry = Map::new();
            entry.insert("node".to_owned(), Value::String(id.to_string()));
            if let Some(node) = tree.get(&id) {
                entry.insert("item".to_owned(), Value::String(node.row.item.clone()));
            }
            ready.push(Value::Object(entry));
        }
        let mut result = Map::new();
        // Ready rather than unclaimed: a node whose dependencies are not
        // done is not work anybody can take, and offering it would send
        // a run to a door that is locked.
        result.insert("ready".to_owned(), Value::Array(ready));
        result.insert("in_progress".to_owned(), Value::Array(working));
        result.insert("blocked".to_owned(), Value::Array(red));
        // The denominator is on screen for the person; it is here so the
        // model does not have to count the array to know where it stands.
        result.insert(
            "nodes_total".to_owned(),
            Value::Number(u64::try_from(tree.len()).unwrap_or(u64::MAX).into()),
        );
        Payload::new(result)
    }

    fn claim(&mut self, id: &NodeId) -> Result<Payload, AxError> {
        if let Some(already) = self.holding() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "claim a plan node",
                format!("this run already holds {already}"),
            )
            .with_recovery(format!(
                "finish, block or release {already} first; a run works on one node"
            )));
        }
        let tree = self.tree()?;
        let held = tree.claim(id)?;
        let item = tree
            .get(id)
            .map_or_else(String::new, |node| node.row.item.clone());
        self.text = set_roadmap_status(&self.text, id, RoadmapStatus::InProgress, None)?;
        self.changed = true;
        self.effects.push(ClaimEffect::Claimed {
            id: id.clone(),
            item: item.clone(),
        });
        self.held = Some(held);
        let mut result = Map::new();
        result.insert("node".to_owned(), Value::String(id.to_string()));
        result.insert("item".to_owned(), Value::String(item));
        result.insert("held_by".to_owned(), Value::String(self.who.clone()));
        Payload::new(result)
    }

    /// The one door out of a held node that needs a locator; the other
    /// two go through [`Self::put_down`].
    fn finish(&mut self, id: &NodeId, raw_evidence: &str) -> Result<Payload, AxError> {
        let evidence = Locator::parse(raw_evidence)?;
        let held = self.take_held(id)?;
        self.record(held.finish(evidence.clone()))?;
        let mut result = Map::new();
        result.insert("node".to_owned(), Value::String(id.to_string()));
        result.insert("evidence".to_owned(), Value::String(evidence.to_string()));
        Payload::new(result)
    }

    fn put_down(&mut self, id: &NodeId, why: StopCause) -> Result<Payload, AxError> {
        let held = self.take_held(id)?;
        let red = why.is_red();
        let line = why.line();
        self.record(held.stop(why))?;
        let mut result = Map::new();
        result.insert("node".to_owned(), Value::String(id.to_string()));
        result.insert("red".to_owned(), Value::Bool(red));
        result.insert("why".to_owned(), Value::String(line));
        Payload::new(result)
    }

    /// Divides a node into children and leaves the run holding nothing:
    /// the work it took has become several pieces, and one of them is
    /// what it should take next.
    fn split(&mut self, id: &NodeId, children: &[NewChild]) -> Result<Payload, AxError> {
        let tree = self.tree()?;
        if tree.get(id).is_none() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "split a plan node",
                format!("no node numbered {id}"),
            )
            .with_recovery("list the plan first and use an index it carries"));
        }
        let grown = insert_children(&self.text, id, children)?;
        // Built before it is written: a split that would not parse, or
        // that would push the plan past its depth, is refused with the
        // file untouched rather than repaired afterwards.
        let RoadmapShape::WellFormed { rows } = check_roadmap_shape(&grown) else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "split a plan node",
                "the split would leave a table that does not parse",
            )
            .with_recovery("shorten the child items; they must fit one table row each"));
        };
        PlanTree::build(rows)?;
        self.text = grown;
        self.changed = true;
        if self.holding() == Some(id) {
            self.held = None;
        }
        let names: Vec<String> = children
            .iter()
            .map(|child| child.item.trim().to_owned())
            .collect();
        self.effects.push(ClaimEffect::Split {
            parent: id.clone(),
            children: names.clone(),
        });
        let mut result = Map::new();
        result.insert("node".to_owned(), Value::String(id.to_string()));
        result.insert(
            "children".to_owned(),
            Value::Array(names.into_iter().map(Value::String).collect()),
        );
        Payload::new(result)
    }

    /// Takes the held node, refusing when it is not the one named.
    ///
    /// The run may only put down what it took: a run that could close
    /// somebody else's node could hand out evidence for work it never
    /// saw.
    fn take_held(&mut self, id: &NodeId) -> Result<Held, AxError> {
        match self.held.take() {
            Some(held) if held.id() == id => Ok(held),
            Some(held) => {
                let holding = held.id().clone();
                self.held = Some(held);
                Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "put down a plan node",
                    format!("this run holds {holding}, not {id}"),
                )
                .with_recovery(format!("put down {holding}, or claim {id} first")))
            }
            None => Err(AxError::failure(
                AxCode::InvalidArgs,
                "put down a plan node",
                format!("this run holds nothing, so it cannot put down {id}"),
            )
            .with_recovery("claim it first; a node is closed by the run that took it")),
        }
    }
}

mod tool;

pub use tool::ClaimTool;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
