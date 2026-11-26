// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use crate::claim_effect::ClaimEffect;
use kernel::spine::check_roadmap_shape;
use kernel::{
    Address, AxCode, AxError, EvidenceCell, Held, Locator, NewChild, NodeId, Payload, PlanExit,
    PlanTree, RoadmapShape, RoadmapStatus, StopCause,
};
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct ClaimDesk {
    who: String,
    room: Address,
    text: String,
    dispatched: String,
    changed: bool,

    held: Option<Held>,
    effects: Vec<ClaimEffect>,
    booking: Booking,
}

pub struct Booking(Box<Ask>);

type Ask = dyn FnMut(&ClaimEffect) -> Result<(), AxError> + Send;

impl Booking {
    #[must_use]
    pub fn new(ask: impl FnMut(&ClaimEffect) -> Result<(), AxError> + Send + 'static) -> Booking {
        Booking(Box::new(ask))
    }
}

pub struct Pinner(Box<Pin>);

type Pin = dyn Fn(&str) -> Result<Locator, AxError> + Send + Sync;

impl Pinner {
    #[must_use]
    pub fn new(pin: impl Fn(&str) -> Result<Locator, AxError> + Send + Sync + 'static) -> Pinner {
        Pinner(Box::new(pin))
    }

    pub(crate) fn evidence(pinner: Option<&Pinner>, evidence: &str) -> Result<Locator, AxError> {
        let spelled_as_locator = ["cas:", "file:"]
            .iter()
            .any(|scheme| evidence.starts_with(scheme));
        match (Locator::parse(evidence), pinner) {
            (Ok(locator), _) => Ok(locator),
            (Err(refused), None) => Err(refused),
            (Err(refused), Some(_)) if spelled_as_locator => Err(refused),
            (Err(_), Some(pinner)) => (pinner.0)(evidence),
        }
    }
}

impl std::fmt::Debug for Pinner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pinner")
    }
}

impl std::fmt::Debug for Booking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Booking")
    }
}

impl ClaimDesk {
    #[must_use]
    pub fn new(who: String, room: Address, roadmap: String, booking: Booking) -> ClaimDesk {
        ClaimDesk {
            who,
            room,
            dispatched: roadmap.clone(),
            text: roadmap,
            changed: false,
            held: None,
            effects: Vec::new(),
            booking,
        }
    }

    pub fn take_effects(&mut self) -> Vec<ClaimEffect> {
        std::mem::take(&mut self.effects)
    }

    #[must_use]
    pub fn roadmap(&self) -> Option<&str> {
        if self.changed { Some(&self.text) } else { None }
    }

    #[must_use]
    pub fn who(&self) -> &str {
        &self.who
    }

    #[must_use]
    pub fn holding(&self) -> Option<&NodeId> {
        self.held.as_ref().map(Held::id)
    }

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

    fn record(&mut self, exit: PlanExit) -> Result<(), AxError> {
        let tree = self.tree()?;
        let item = tree
            .get(exit.id())
            .map_or_else(String::new, |node| node.row.item.clone());
        self.take(ClaimEffect::PutDown {
            id: exit.id().clone(),
            item,
            exit,
        })
    }

    fn take(&mut self, effect: ClaimEffect) -> Result<(), AxError> {
        self.text = effect.apply(&self.text)?;
        self.changed = true;
        self.effects.push(effect);
        Ok(())
    }

    fn list(&self) -> Result<Payload, AxError> {
        let tree = self.tree()?;
        let mut ready = Vec::new();
        let mut working = Vec::new();
        let mut red = Vec::new();
        let mut done = Vec::new();
        for node in tree.nodes() {
            let mut entry = Map::new();
            entry.insert("node".to_owned(), Value::String(node.row.id.to_string()));
            entry.insert("item".to_owned(), Value::String(node.row.item.clone()));
            match node.row.status {
                RoadmapStatus::InProgress if node.is_leaf() => working.push(Value::Object(entry)),
                RoadmapStatus::Blocked | RoadmapStatus::AwaitingApproval => {
                    red.push(Value::Object(entry));
                }

                RoadmapStatus::Done => {
                    if let EvidenceCell::Present(evidence) = &node.row.evidence {
                        entry.insert("evidence".to_owned(), Value::String(evidence.to_string()));
                    }
                    done.push(Value::Object(entry));
                }
                RoadmapStatus::InProgress | RoadmapStatus::NotStarted => {}
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

        result.insert("ready".to_owned(), Value::Array(ready));
        result.insert("in_progress".to_owned(), Value::Array(working));
        result.insert("blocked".to_owned(), Value::Array(red));
        result.insert("done".to_owned(), Value::Array(done));
        result.insert("pending".to_owned(), Value::Array(self.pending(&tree)));

        result.insert(
            "nodes_total".to_owned(),
            Value::Number(u64::try_from(tree.len()).unwrap_or(u64::MAX).into()),
        );
        Payload::new(result)
    }

    fn pending(&self, tree: &PlanTree) -> Vec<Value> {
        if !self.changed {
            return Vec::new();
        }
        let RoadmapShape::WellFormed { rows } = check_roadmap_shape(&self.dispatched) else {
            return Vec::new();
        };
        let before: BTreeMap<_, _> = rows.iter().map(|row| (&row.id, row)).collect();
        tree.nodes()
            .filter(|node| before.get(&node.row.id).copied() != Some(&node.row))
            .map(|node| Value::String(node.row.id.to_string()))
            .collect()
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
        let claimed = ClaimEffect::Claimed {
            id: id.clone(),
            item: item.clone(),
        };

        (self.booking.0)(&claimed)?;
        self.take(claimed)?;
        self.held = Some(held);
        let mut result = Map::new();
        result.insert("node".to_owned(), Value::String(id.to_string()));
        result.insert("item".to_owned(), Value::String(item));
        result.insert("held_by".to_owned(), Value::String(self.who.clone()));
        Payload::new(result)
    }

    fn finish(&mut self, id: &NodeId, evidence: Locator) -> Result<Payload, AxError> {
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

    fn split(&mut self, id: &NodeId, children: &[NewChild]) -> Result<Payload, AxError> {
        self.refuse_unless_holding(id)?;
        let effect = ClaimEffect::Split {
            parent: id.clone(),
            children: children
                .iter()
                .map(|child| NewChild {
                    item: child.item.trim().to_owned(),
                    weight: child.weight,
                    needs: child.needs.clone(),
                })
                .collect(),
        };
        let grown = effect.apply(&self.text)?;

        let RoadmapShape::WellFormed { rows } = check_roadmap_shape(&grown) else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "split a plan node",
                "the split would leave a table that does not parse",
            )
            .with_recovery("shorten the child items; they must fit one table row each"));
        };
        let after = PlanTree::build(rows)?;
        let unfinished = after.get(id).map_or(0, |parent| {
            parent
                .children
                .iter()
                .filter_map(|child| after.get(child))
                .filter(|child| child.row.status != RoadmapStatus::Done)
                .count()
        });
        let names: Vec<String> = children
            .iter()
            .map(|child| child.item.trim().to_owned())
            .collect();
        self.text = grown;
        self.changed = true;
        self.effects.push(effect);
        self.held = None;
        let mut result = Map::new();
        result.insert("node".to_owned(), Value::String(id.to_string()));
        result.insert(
            "children".to_owned(),
            Value::Array(names.into_iter().map(Value::String).collect()),
        );
        result.insert("unfinished".to_owned(), Value::from(unfinished));
        Payload::new(result)
    }

    fn refuse_unless_holding(&self, id: &NodeId) -> Result<(), AxError> {
        let refuse = |subject: String, recovery: String| {
            AxError::failure(AxCode::InvalidArgs, "split a plan node", subject)
                .with_recovery(recovery)
        };
        match self.holding() {
            Some(held) if held == id => Ok(()),
            Some(held) => Err(refuse(
                format!("this run holds {held}, not {id}"),
                format!("split {held}, or put it down and claim {id} first"),
            )),
            None => Err(refuse(
                format!("this run holds nothing, so it cannot split {id}"),
                format!("claim {id} first; a row is divided by the run that holds it"),
            )),
        }
    }

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

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod booking_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod catalog_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod split_tests;
