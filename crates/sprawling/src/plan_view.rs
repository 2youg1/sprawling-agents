// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every building's plan, parsed once and re-parsed only when something
//! could have moved it.
//!
//! **The file is still the plan.** What changed is who does the reading:
//! `CityView` and `Metrics` used to open every building's `Roadmap.md`
//! and parse it again for every question a page asked, which is a disk
//! read and a parse per poll for a document that changes a few times an
//! hour. `kernel::WriteMoment` says the plan may only be written at
//! three moments, and every one of those moments is an event — so this
//! folds the events and re-reads a building only when one of them
//! names it.
//!
//! **Why it is a projection and not a copy.** Nothing here stores what
//! the plan says; it stores what the plan *was* the last time it was
//! read, and throws that away the moment anything could have changed
//! it. Deleting the whole thing and folding the ledger again gives the
//! same bytes, because folding is all it does — that is the property
//! the test at the bottom pins.
//!
//! It also folds the one fact the file cannot carry: **why** a node is
//! red. The table has room to say `Blocked`; the sentence a person needs
//! is in the `roadmap_blocked` record, and putting a second copy of it
//! in the table would be a second authority for the same sentence.

use std::collections::BTreeMap;
use std::path::Path;

use kernel::{
    Address, Blockage, EventKind, EventRecord, NodeId, PlanTree, Progress, RedNode, RoadmapShape,
    RoadmapStatus, StopCause, UnplannedProgress,
};

/// What one building's plan came to when it was last read.
enum Reading {
    Tree(Box<PlanTree>),
    /// The plan is there and cannot be read as one. Kept rather than
    /// retried, so a broken table costs one parse rather than one per
    /// question.
    Unreadable(Vec<String>),
}

/// The plans of a city.
#[derive(Default)]
pub(crate) struct PlanView {
    read: BTreeMap<Address, Reading>,
    /// Why each red node is red, folded from the records that said so.
    causes: BTreeMap<Address, BTreeMap<NodeId, StopCause>>,
}

/// What a page is told about one building's plan.
pub(crate) struct PlanReading {
    pub(crate) progress: Progress,
    pub(crate) problems: Vec<String>,
    pub(crate) rows: Vec<channels::PlanRow>,
    pub(crate) blocked: Vec<channels::BlockedLine>,
    pub(crate) ready: Vec<NodeId>,
}

impl PlanView {
    /// Folds one record.
    ///
    /// Two jobs, and they are separate on purpose. A `roadmap_*` record
    /// says what a run did to the plan, which is both a reason to forget
    /// the parsed copy and a fact worth keeping. A checkpoint says a
    /// tool wave wrote files, which is only the first: an agent that
    /// edited the table with the edit tool leaves no `roadmap_*` record,
    /// and a cache that ignored the wave would go on reporting the plan
    /// as it was before the edit.
    pub(crate) fn apply(&mut self, record: &EventRecord) {
        let Some(building) = record.addr().and_then(building_of) else {
            // A record with no address could belong to any building, so
            // every parsed copy is suspect.
            if matches!(record.kind(), EventKind::CityInitialized) {
                self.read.clear();
            }
            return;
        };
        match record.kind() {
            EventKind::RoadmapClaimed => self.read.remove(&building).map(drop).unwrap_or(()),
            EventKind::RoadmapFinished | EventKind::RoadmapReleased => {
                self.read.remove(&building);
                if let Some(node) = node_of(record) {
                    self.causes.entry(building).or_default().remove(&node);
                }
            }
            EventKind::RoadmapBlocked => {
                self.read.remove(&building);
                if let (Some(node), Some(why)) = (node_of(record), cause_of(record)) {
                    self.causes.entry(building).or_default().insert(node, why);
                }
            }
            EventKind::RoadmapSplit
            | EventKind::CheckpointCommitted
            | EventKind::BuildingCreated
            | EventKind::RunFrozen => {
                self.read.remove(&building);
            }
            _ => {}
        }
    }

    /// What one building's plan says, reading the file only when the
    /// fold says it may have moved.
    pub(crate) fn of(&mut self, city_root: &Path, addr: &Address) -> PlanReading {
        if !self.read.contains_key(addr) {
            let reading = match city::roadmap(city_root, addr) {
                // A plan that cannot be opened is not a plan somebody
                // wrote badly. Read as an empty document it would come
                // back as "no table found", which sends a person to
                // edit a table when the file will not open.
                Err(err) => Reading::Unreadable(vec![err.to_string()]),
                Ok(text) => match kernel::check_roadmap_shape(&text) {
                    RoadmapShape::WellFormed { rows } => match PlanTree::build(rows) {
                        Ok(tree) => Reading::Tree(Box::new(tree)),
                        Err(refusal) => Reading::Unreadable(vec![refusal.to_string()]),
                    },
                    RoadmapShape::Malformed { problems } => Reading::Unreadable(problems),
                },
            };
            self.read.insert(addr.clone(), reading);
        }
        match self.read.get(addr) {
            Some(Reading::Tree(tree)) => self.describe(addr, tree),
            Some(Reading::Unreadable(problems)) => PlanReading {
                progress: unplanned(),
                problems: problems.clone(),
                rows: Vec::new(),
                blocked: Vec::new(),
                ready: Vec::new(),
            },
            None => PlanReading {
                progress: unplanned(),
                problems: Vec::new(),
                rows: Vec::new(),
                blocked: Vec::new(),
                ready: Vec::new(),
            },
        }
    }

    fn describe(&self, addr: &Address, tree: &PlanTree) -> PlanReading {
        let ready = tree.ready();
        let rows = tree
            .nodes()
            .map(|node| channels::PlanRow {
                node: node.row.id.clone(),
                item: node.row.item.clone(),
                status: node.row.status,
                share_ppb: node.share.ppb(),
                needs: node.row.needs.clone(),
                ready: ready.contains(&node.row.id),
                leaf: node.is_leaf(),
                evidence: match &node.row.evidence {
                    kernel::EvidenceCell::Present(locator) => Some(locator.to_string()),
                    kernel::EvidenceCell::Empty | kernel::EvidenceCell::Invalid { .. } => None,
                },
            })
            .collect();
        let blocked = self
            .blockages(addr, tree)
            .into_iter()
            .map(|blockage| channels::BlockedLine {
                line: blockage.line(),
                waiting: u32::try_from(blockage.reaches.len()).unwrap_or(u32::MAX),
                source: blockage.source,
            })
            .collect();
        PlanReading {
            progress: tree.progress(),
            problems: Vec::new(),
            rows,
            blocked,
            ready,
        }
    }

    /// The red nodes of one building, and how far each one reaches.
    ///
    /// The table says which nodes are red; the fold says why. A node the
    /// table calls blocked with no record behind it is still red — the
    /// sentence is then the status word itself, because a row a person
    /// edited by hand is still a row that says the work has stopped.
    fn blockages(&self, addr: &Address, tree: &PlanTree) -> Vec<Blockage> {
        let known = self.causes.get(addr);
        let red: Vec<RedNode> = tree
            .nodes()
            .filter(|node| {
                matches!(
                    node.row.status,
                    RoadmapStatus::Blocked | RoadmapStatus::AwaitingApproval
                )
            })
            .map(|node| RedNode {
                at: node.row.id.clone(),
                why: known
                    .and_then(|held| held.get(&node.row.id))
                    .cloned()
                    .unwrap_or_else(|| StopCause::Blocked {
                        note: format!("the plan says `{}`", node.row.status.spelling()),
                    }),
            })
            .collect();
        kernel::spread(tree, &red)
    }
}

fn unplanned() -> Progress {
    Progress::Unplanned(UnplannedProgress {
        steps: 0,
        budget: kernel::BudgetUse::default(),
    })
}

/// The building an address belongs to: its first segment.
fn building_of(addr: &Address) -> Option<Address> {
    let head = addr.as_str().split('/').next()?;
    Address::parse(head).ok()
}

fn node_of(record: &EventRecord) -> Option<NodeId> {
    NodeId::parse(record.data().as_map().get("node")?.as_str()?).ok()
}

fn cause_of(record: &EventRecord) -> Option<StopCause> {
    serde_json::from_value(record.data().as_map().get("why")?.clone()).ok()
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
