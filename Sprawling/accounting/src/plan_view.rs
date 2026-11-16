// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Mutex;

use kernel::{
    Address, Blockage, EventRecord, NodeId, PlanTree, Progress, RedNode, RoadmapShape,
    RoadmapStatus, StopCause, UnplannedProgress,
};

mod reach;
use reach::{PlanReach, building_of, cause_of, may_move_plan, node_of};

enum Reading {
    Tree(Box<PlanTree>),

    Unreadable(Vec<String>),
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct PlanView {
    #[serde(skip)]
    read: BTreeMap<Address, Reading>,

    causes: BTreeMap<Address, BTreeMap<NodeId, StopCause>>,

    #[serde(skip)]
    moved: BTreeMap<Address, u64>,

    #[serde(skip)]
    moved_all: u64,
}

const GENERATIONS_HELD: usize = 1024;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Generation {
    all: u64,
    one: u64,
}

struct FreshPlan {
    addr: Address,
    asked_at: Generation,
    reading: Reading,
}

pub struct PlanReading {
    pub progress: Progress,
    pub problems: Vec<String>,
    pub rows: Vec<wire::PlanRow>,
    pub blocked: Vec<wire::BlockedLine>,
    pub ready: Vec<NodeId>,
}

impl PlanView {

    pub fn apply(&mut self, record: &EventRecord) {
        let reach = may_move_plan(record.kind());
        let Some(building) = record.addr().and_then(building_of) else {

            if !matches!(reach, PlanReach::Untouched) {
                self.read.clear();
                self.moved_all = self.moved_all.wrapping_add(1);
            }
            return;
        };
        if !matches!(reach, PlanReach::Untouched) {
            if self.moved.len() >= GENERATIONS_HELD && !self.moved.contains_key(&building) {
                self.forget_generations();
            }
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

    pub fn of(&mut self, city_root: &Path, addr: &Address) -> PlanReading {
        let (reading, fresh) = self.ask(addr).read(city_root, addr);
        if let Some(fresh) = fresh {
            self.remember(fresh);
        }
        reading
    }

    fn ask(&self, addr: &Address) -> PlanAsk {
        match self.read.get(addr) {
            Some(reading) => PlanAsk::Held(describe(reading, self.causes.get(addr))),
            None => PlanAsk::Unread {
                causes: self.causes.get(addr).cloned().unwrap_or_default(),
                asked_at: self.generation(addr),
            },
        }
    }

    fn remember(&mut self, fresh: FreshPlan) {
        if self.generation(&fresh.addr) == fresh.asked_at {
            self.read.insert(fresh.addr, fresh.reading);
        }
    }

    fn forget_generations(&mut self) {
        self.moved.clear();
        self.moved_all = self.moved_all.wrapping_add(1);
    }

    fn generation(&self, addr: &Address) -> Generation {
        Generation {
            all: self.moved_all,
            one: self.moved.get(addr).copied().unwrap_or_default(),
        }
    }
}

pub fn plans_of(
    shared: &Mutex<PlanView>,
    city_root: &Path,
    addrs: BTreeSet<Address>,
) -> BTreeMap<Address, PlanReading> {
    let asks: Vec<(Address, PlanAsk)> = {
        let view = PlanView::take_back(shared);
        addrs
            .into_iter()
            .map(|addr| {
                let ask = view.ask(&addr);
                (addr, ask)
            })
            .collect()
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
    let mut view = PlanView::take_back(shared);
    for read in fresh {
        view.remember(read);
    }
    readings
}

impl PlanView {

    pub fn take_back(shared: &Mutex<Self>) -> std::sync::MutexGuard<'_, Self> {
        shared.lock().unwrap_or_else(|poisoned| {
            let mut view = poisoned.into_inner();
            view.read.clear();
            view.forget_generations();
            shared.clear_poison();
            view
        })
    }
}

enum PlanAsk {

    Held(PlanReading),

    Unread {
        causes: BTreeMap<NodeId, StopCause>,
        asked_at: Generation,
    },
}

impl PlanAsk {

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

fn read_plan(city_root: &Path, addr: &Address) -> Reading {
    match city::roadmap(city_root, addr) {

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
        .map(|node| wire::PlanRow {
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
        .map(|blockage| wire::BlockedLine {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
