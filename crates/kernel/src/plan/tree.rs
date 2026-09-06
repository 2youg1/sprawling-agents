// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The plan as a tree: placement, division, queries, claims, and progress.

use std::collections::{BTreeMap, BTreeSet};

use crate::completion::{PlannedProgress, Progress};
use crate::error::{AxCode, AxError};
use crate::node_id::NodeId;
use crate::share::{self, Share};
use crate::spine::{EvidenceCell, RoadmapRow, RoadmapStatus};

use super::PlanTree;
use super::node::{Held, PlanNode};

impl PlanTree {
    /// Places every row and works out what each one is worth.
    ///
    /// # Errors
    /// Fail-closed, in the same shape as `crate::locator`: a plan that
    /// cannot be read is refused rather than guessed at, because every
    /// progress figure in the city is divided by it. Five refusals — a
    /// repeated index, a node whose parent is missing, a dependency on a
    /// node that is not there, a dependency in a circle, and a branch
    /// marked done over a child that is not.
    pub fn build(rows: Vec<RoadmapRow>) -> Result<PlanTree, AxError> {
        let mut nodes: BTreeMap<NodeId, PlanNode> = BTreeMap::new();
        for row in rows {
            let id = row.id.clone();
            if nodes.contains_key(&id) {
                return Err(super::blocking::refusal(
                    "place a plan node",
                    format!("index {id} appears twice"),
                    "give every row its own index; two rows with one number are two plans",
                ));
            }
            nodes.insert(
                id,
                PlanNode {
                    row,
                    share: Share::NONE,
                    children: Vec::new(),
                },
            );
        }
        let placed: Vec<NodeId> = nodes.keys().cloned().collect();
        for id in &placed {
            if let Some(parent) = id.parent() {
                let Some(held) = nodes.get_mut(&parent) else {
                    return Err(super::blocking::refusal(
                        "place a plan node",
                        format!("{id} hangs under {parent}, which the table does not carry"),
                        "write the parent row first; a plan cannot branch from nothing",
                    ));
                };
                held.children.push(id.clone());
            }
        }
        Self::check_dependencies(&nodes)?;
        Self::check_branch_claims(&nodes)?;
        let mut tree = PlanTree { nodes };
        tree.divide()?;
        Ok(tree)
    }

    /// Every dependency names a node that exists, is not the node
    /// itself, and does not close a circle.
    fn check_dependencies(nodes: &BTreeMap<NodeId, PlanNode>) -> Result<(), AxError> {
        for (id, node) in nodes {
            for need in &node.row.needs {
                if !nodes.contains_key(need) {
                    return Err(super::blocking::refusal(
                        "read a plan dependency",
                        format!("{id} needs {need}, which the table does not carry"),
                        "name a row the plan holds, or drop the dependency",
                    ));
                }
                if need == id {
                    return Err(super::blocking::refusal(
                        "read a plan dependency",
                        format!("{id} needs itself"),
                        "a node cannot wait for its own result",
                    ));
                }
            }
        }
        if let Some(circle) = super::blocking::first_cycle(nodes) {
            let drawn: Vec<String> = circle.iter().map(NodeId::to_string).collect();
            return Err(super::blocking::refusal(
                "read a plan dependency",
                format!("{} runs in a circle", drawn.join(" → ")),
                "drop one of those dependencies; nothing in a circle can ever start",
            ));
        }
        Ok(())
    }

    /// A branch that says `Done` over a child that is not is refused,
    /// which is what lets the status column of a branch be read as a
    /// summary instead of as a second opinion.
    fn check_branch_claims(nodes: &BTreeMap<NodeId, PlanNode>) -> Result<(), AxError> {
        for (id, node) in nodes {
            if node.children.is_empty() || node.row.status != RoadmapStatus::Done {
                continue;
            }
            for child in &node.children {
                let unfinished = nodes
                    .get(child)
                    .is_some_and(|held| held.row.status != RoadmapStatus::Done);
                if unfinished {
                    return Err(super::blocking::refusal(
                        "read a plan branch",
                        format!("{id} says done while {child} does not"),
                        "a branch is done when its children are; finish the child or reopen the \
                         branch",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Divides the whole plan down the tree, top-level rows first.
    fn divide(&mut self) -> Result<(), AxError> {
        let roots: Vec<NodeId> = self
            .nodes
            .keys()
            .filter(|id| id.parent().is_none())
            .cloned()
            .collect();
        super::share::hand_out(self, Share::WHOLE, &roots)?;
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &NodeId) -> Option<&PlanNode> {
        self.nodes.get(id)
    }

    /// Every node, in table order.
    pub fn nodes(&self) -> impl Iterator<Item = &PlanNode> {
        self.nodes.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// What a node waits for, including everything its ancestors wait
    /// for. A child of a branch that is waiting is waiting too;
    /// answering otherwise would let `2.3.1` start while the reason
    /// `2.3` cannot start is still unresolved.
    #[must_use]
    pub fn needs_of(&self, id: &NodeId) -> BTreeSet<NodeId> {
        let mut found = BTreeSet::new();
        for step in id
            .ancestors()
            .into_iter()
            .chain(std::iter::once(id.clone()))
        {
            if let Some(node) = self.nodes.get(&step) {
                found.extend(node.row.needs.iter().cloned());
            }
        }
        found
    }

    /// What may be started right now: leaves nobody has taken, whose
    /// dependencies — their own and their branch's — are all done.
    ///
    /// Pure, and that is the point: a city with no ready nodes and
    /// nothing running has finished, and a stop condition that depended
    /// on I/O could not be trusted to say so.
    #[must_use]
    pub fn ready(&self) -> Vec<NodeId> {
        self.nodes
            .values()
            .filter(|node| node.is_leaf() && node.row.status == RoadmapStatus::NotStarted)
            .filter(|node| {
                self.needs_of(&node.row.id).iter().all(|need| {
                    self.nodes
                        .get(need)
                        .is_some_and(|held| held.row.status == RoadmapStatus::Done)
                })
            })
            .map(|node| node.row.id.clone())
            .collect()
    }

    /// Takes a node to work on.
    ///
    /// # Errors
    /// Refuses everything [`Self::ready`] leaves out, and says which of
    /// the three reasons applies: the index is not in the table, the
    /// node is a branch or already taken, or it is still waiting on
    /// something. The third names what it waits for, because "no" sends
    /// a model round the loop again while "1.4 is not done" does not.
    pub fn claim(&self, id: &NodeId) -> Result<Held, AxError> {
        let node = self.nodes.get(id).ok_or_else(|| {
            super::blocking::refusal(
                "claim a plan node",
                format!("no row numbered {id}"),
                format!("list the plan first; it carries {} rows", self.nodes.len()),
            )
        })?;
        if !node.is_leaf() {
            return Err(super::blocking::refusal(
                "claim a plan node",
                format!("{id} is a branch with {} children", node.children.len()),
                "claim one of its children; a branch is done when its children are",
            ));
        }
        if node.row.status != RoadmapStatus::NotStarted {
            // Two runs wanting one node is a goal conflict, and the
            // caller reads the code rather than the sentence.
            return Err(AxError::failure(
                AxCode::GoalConflict,
                "claim a plan node",
                format!("{id} is `{}`", node.row.status.spelling()),
            )
            .with_recovery(self.somewhere_else()));
        }
        let waiting: Vec<String> = self
            .needs_of(id)
            .into_iter()
            .filter(|need| {
                self.nodes
                    .get(need)
                    .is_none_or(|held| held.row.status != RoadmapStatus::Done)
            })
            .map(|need| need.to_string())
            .collect();
        if !waiting.is_empty() {
            return Err(super::blocking::refusal(
                "claim a plan node",
                format!("{id} waits for {}", waiting.join(", ")),
                self.somewhere_else(),
            ));
        }
        Ok(Held::of(id.clone()))
    }

    /// The third part of a refusal: a node the caller may actually take.
    fn somewhere_else(&self) -> String {
        self.ready().first().map_or_else(
            || "nothing is ready; report to the person rather than picking a node".to_owned(),
            |id| format!("claim {id}, which is ready"),
        )
    }

    /// The plan's own reading of itself: leaves counted, and the share
    /// of the whole those leaves carry.
    ///
    /// Both figures travel because they answer different questions and
    /// mislead alone. A share says how much of the plan is behind you; a
    /// leaf count says how many pieces the plan turned out to have, and
    /// it is the figure that does not move when somebody divides their
    /// own branch generously.
    #[must_use]
    pub fn progress(&self) -> Progress {
        let mut done: u32 = 0;
        let mut blocked: u32 = 0;
        let mut total: u32 = 0;
        let mut done_parts = Vec::new();
        let mut blocked_parts = Vec::new();
        for node in self.nodes.values() {
            if !node.is_leaf() {
                continue;
            }
            total = total.saturating_add(1);
            match (&node.row.status, &node.row.evidence) {
                (RoadmapStatus::Done, EvidenceCell::Present(_)) => {
                    done = done.saturating_add(1);
                    done_parts.push(node.share);
                }
                (RoadmapStatus::Blocked | RoadmapStatus::AwaitingApproval, _) => {
                    blocked = blocked.saturating_add(1);
                    blocked_parts.push(node.share);
                }
                _ => {}
            }
        }
        Progress::Planned(PlannedProgress {
            done,
            blocked,
            total,
            done_ppb: share::gather(&done_parts).ppb(),
            blocked_ppb: share::gather(&blocked_parts).ppb(),
        })
    }
}

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
