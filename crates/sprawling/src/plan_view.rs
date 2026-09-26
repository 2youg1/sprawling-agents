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

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Mutex;

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
    /// How many records may have moved each building's plan. A plan
    /// read with the cache released is put back only when this has not
    /// moved since it was asked for.
    moved: BTreeMap<Address, u64>,
    /// How many records with no address may have moved every plan.
    moved_all: u64,
}

/// Where the fold stood on one building's plan when a reader asked for
/// it: equal again at the write-back when no record moved that plan in
/// between.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Generation {
    all: u64,
    one: u64,
}

/// A plan read off the disk with the cache released, waiting to be put
/// back.
struct FreshPlan {
    addr: Address,
    asked_at: Generation,
    reading: Reading,
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
        let reach = may_move_plan(record.kind());
        let Some(building) = record.addr().and_then(building_of) else {
            // A record with no address could belong to any building, so
            // a kind that moves a plan makes every parsed copy suspect.
            // The rule used to name one kind and the comment beside it
            // named the class, and the comment was the correct one
            // (sprawling-SPEC.md 8-76).
            if !matches!(reach, PlanReach::Untouched) {
                self.read.clear();
                self.moved_all = self.moved_all.wrapping_add(1);
            }
            return;
        };
        if !matches!(reach, PlanReach::Untouched) {
            let moved = self.moved.entry(building.clone()).or_default();
            *moved = moved.wrapping_add(1);
        }
        match reach {
            PlanReach::Untouched => {}
            PlanReach::Stale => {
                self.read.remove(&building);
            }
            PlanReach::NodeFreed => {
                self.read.remove(&building);
                if let Some(node) = node_of(record) {
                    self.causes.entry(building).or_default().remove(&node);
                }
            }
            PlanReach::NodeStopped => {
                self.read.remove(&building);
                if let (Some(node), Some(why)) = (node_of(record), cause_of(record)) {
                    self.causes.entry(building).or_default().insert(node, why);
                }
            }
        }
    }

    /// What one building's plan says, reading the file only when the
    /// fold says it may have moved: the three steps [`plans_of`] takes
    /// around its lock, taken in one.
    #[cfg(test)]
    pub(crate) fn of(&mut self, city_root: &Path, addr: &Address) -> PlanReading {
        let (reading, fresh) = self.ask(addr).read(city_root, addr);
        if let Some(fresh) = fresh {
            self.remember(fresh);
        }
        reading
    }

    /// What one building's plan says as far as the cache holds it,
    /// without reading the disk: the cache is held while this runs, and
    /// a plan nobody has read yet is left to [`PlanAsk::read`].
    fn ask(&self, addr: &Address) -> PlanAsk {
        match self.read.get(addr) {
            Some(reading) => PlanAsk::Held(describe(reading, self.causes.get(addr))),
            None => PlanAsk::Unread {
                causes: self.causes.get(addr).cloned().unwrap_or_default(),
                asked_at: self.generation(addr),
            },
        }
    }

    /// Puts back a plan read with the cache released, unless a record
    /// folded since it was asked for may have moved it.
    fn remember(&mut self, fresh: FreshPlan) {
        self.read.insert(fresh.addr, fresh.reading);
    }

    fn generation(&self, addr: &Address) -> Generation {
        Generation {
            all: self.moved_all,
            one: self.moved.get(addr).copied().unwrap_or_default(),
        }
    }
}

/// Every named building's plan: described from the cache while it is
/// held, read off the disk once it is released, and put back only when
/// no record moved that plan in between (sprawling-SPEC.md 8-92).
///
/// A poisoned cache is not consulted again: the fold stopped moving it
/// when it was poisoned, so every plan is read off the disk, and a red
/// node's sentence falls back to the status word the table carries.
pub(crate) fn plans_of(
    shared: &Mutex<PlanView>,
    city_root: &Path,
    addrs: BTreeSet<Address>,
) -> BTreeMap<Address, PlanReading> {
    let asks: Vec<(Address, PlanAsk)> = match shared.lock() {
        Ok(view) => addrs
            .into_iter()
            .map(|addr| {
                let ask = view.ask(&addr);
                (addr, ask)
            })
            .collect(),
        Err(_) => addrs
            .into_iter()
            .map(|addr| (addr, PlanAsk::uncached()))
            .collect(),
    };
    let mut fresh = Vec::new();
    let readings = asks
        .into_iter()
        .map(|(addr, ask)| {
            let (reading, read) = ask.read(city_root, &addr);
            fresh.extend(read);
            (addr, reading)
        })
        .collect();
    match shared.lock() {
        Ok(mut view) => fresh.into_iter().for_each(|read| view.remember(read)),
        // Poisoned: the cache is abandoned, so there is nothing to fill.
        Err(_) => {}
    }
    readings
}

/// One building's plan as `PlanView::ask` left it for after the lock.
enum PlanAsk {
    /// The cached plan, described.
    Held(PlanReading),
    /// A plan nobody has read since it last moved, why each of its red
    /// nodes is red, and where the fold stood when it was asked for.
    Unread {
        causes: BTreeMap<NodeId, StopCause>,
        asked_at: Generation,
    },
}

impl PlanAsk {
    /// A plan read with no cache behind it.
    fn uncached() -> Self {
        Self::Unread {
            causes: BTreeMap::new(),
            asked_at: Generation::default(),
        }
    }

    /// The plan, reading the file when the cache did not hold it, and
    /// what was read, for [`PlanView::remember`] to put back.
    fn read(self, city_root: &Path, addr: &Address) -> (PlanReading, Option<FreshPlan>) {
        match self {
            Self::Held(reading) => (reading, None),
            Self::Unread { causes, asked_at } => {
                let reading = read_plan(city_root, addr);
                (
                    describe(&reading, Some(&causes)),
                    Some(FreshPlan {
                        addr: addr.clone(),
                        asked_at,
                        reading,
                    }),
                )
            }
        }
    }
}

/// One building's plan file, parsed.
fn read_plan(city_root: &Path, addr: &Address) -> Reading {
    match city::roadmap(city_root, addr) {
        // A plan that cannot be opened is not a plan somebody wrote
        // badly. Read as an empty document it would come back as "no
        // table found", which sends a person to edit a table when the
        // file will not open.
        Err(err) => Reading::Unreadable(vec![err.to_string()]),
        Ok(text) => match kernel::spine::check_roadmap_shape(&text) {
            RoadmapShape::WellFormed { rows } => match PlanTree::build(rows) {
                Ok(tree) => Reading::Tree(Box::new(tree)),
                Err(refusal) => Reading::Unreadable(vec![refusal.to_string()]),
            },
            RoadmapShape::Malformed { problems } => Reading::Unreadable(problems),
        },
    }
}

/// What a page is told about one reading, given why its red nodes are red.
fn describe(reading: &Reading, causes: Option<&BTreeMap<NodeId, StopCause>>) -> PlanReading {
    match reading {
        Reading::Tree(tree) => describe_tree(tree, causes),
        Reading::Unreadable(problems) => PlanReading {
            progress: unplanned(),
            problems: problems.clone(),
            rows: Vec::new(),
            blocked: Vec::new(),
            ready: Vec::new(),
        },
    }
}

fn describe_tree(tree: &PlanTree, causes: Option<&BTreeMap<NodeId, StopCause>>) -> PlanReading {
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
    let blocked = blockages(tree, causes)
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
fn blockages(tree: &PlanTree, known: Option<&BTreeMap<NodeId, StopCause>>) -> Vec<Blockage> {
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
    kernel::blockage::spread(tree, &red)
}

fn unplanned() -> Progress {
    Progress::Unplanned(UnplannedProgress {
        steps: 0,
        budget: kernel::BudgetUse::default(),
    })
}

/// How far one record reaches into what this view holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PlanReach {
    /// The kind cannot move a plan, so every parsed copy still stands.
    Untouched,
    /// The plan may have moved and the parsed copy is out of date.
    Stale,
    /// Out of date, and the node the record names is no longer stopped.
    NodeFreed,
    /// Out of date, and the node the record names has stopped, for the
    /// reason the record carries.
    NodeStopped,
}

/// Which records can move a plan. Exhaustive on purpose: a kind added
/// to the vocabulary without an answer here is a compile error rather
/// than a stale table nobody notices (sprawling-SPEC.md 8-76).
fn may_move_plan(kind: EventKind) -> PlanReach {
    match kind {
        EventKind::RoadmapFinished | EventKind::RoadmapReleased => PlanReach::NodeFreed,
        EventKind::RoadmapBlocked => PlanReach::NodeStopped,
        EventKind::CityInitialized
        | EventKind::BuildingCreated
        | EventKind::CheckpointCommitted
        | EventKind::RunFrozen
        | EventKind::RoadmapClaimed
        | EventKind::RoadmapSplit
        // A person's write to `Roadmap.md` moves the plan like any
        // other, so the table is re-read rather than trusted.
        | EventKind::SpineDocumentWritten => PlanReach::Stale,
        EventKind::BuildingConfigured
        | EventKind::SessionOpened
        | EventKind::RunStarted
        | EventKind::RunForked
        | EventKind::PromptAssembled
        // The cache shape measures one request; it names no plan row.
        | EventKind::PromptShapeCompared
        | EventKind::ModelCalled
        | EventKind::ModelReturned
        | EventKind::ToolCalled
        | EventKind::ToolResult
        | EventKind::ResultOffloaded
        | EventKind::GateChecked
        | EventKind::GateDenied
        | EventKind::HandoffWritten
        | EventKind::SteerReceived
        | EventKind::CancelReceived
        | EventKind::WatchdogFired
        | EventKind::BudgetLimit
        | EventKind::LogTruncated
        | EventKind::SignalEnqueued
        | EventKind::SignalConsumed
        | EventKind::DraftHeld
        | EventKind::DraftResolved
        | EventKind::GoalRegistered
        | EventKind::GoalConflict
        | EventKind::ArbitrationVerdict
        | EventKind::RepairStarted
        | EventKind::RepairReused
        | EventKind::WorktreeOpened
        | EventKind::PrOpened
        | EventKind::PrMerged
        | EventKind::PrRejected
        | EventKind::PursuitChanged
        | EventKind::ApprovalRequested
        | EventKind::ApprovalResolved
        | EventKind::PolicyCreated
        | EventKind::PolicyRevoked
        | EventKind::TaintPromoted
        | EventKind::CrossBuildingTransfer
        | EventKind::CityHalted
        | EventKind::BackpressureShed
        | EventKind::DigestInvalidated
        | EventKind::EndpointAttached
        | EventKind::EndpointProbed
        | EventKind::EndpointLost
        | EventKind::ModelSelected
        | EventKind::ProviderDegraded
        | EventKind::LoginStarted
        | EventKind::EvalRun
        | EventKind::AssetArchived
        | EventKind::CredentialLent
        | EventKind::SecretCaptured
        | EventKind::SecretEgressBlocked
        | EventKind::FileDiscarded
        | EventKind::DiscardRestored
        | EventKind::AutonomyChanged
        | EventKind::GovernedDocumentWritten
        | EventKind::RulesChanged
        | EventKind::ToolkitLinkOpened
        // A call to an embeddings or rerank face, and every line an
        // adviser consultation writes, are about the window one run is
        // given: none of them names a plan row.
        | EventKind::EmbeddingCalled
        | EventKind::RerankCalled
        | EventKind::AdviserAsked
        | EventKind::AdviserAnswered
        | EventKind::AdviserFellBack => PlanReach::Untouched,
    }
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
