// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use kernel::{Address, EventKind, EventRecord, RunId, Seq, TimeMs};

use crate::error::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RunPhase {
    Active,
    Frozen,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunHot {
    pub phase: RunPhase,
    pub last_seq: Seq,
    pub last_kind: EventKind,
    pub who: String,

    pub addr: Option<Address>,

    pub started: Option<TimeMs>,

    pub completion: Option<String>,

    pub pr: Option<String>,

    pub ask: Option<String>,

    pub task: Option<String>,

    pub goal: Option<String>,

    #[serde(default)]
    pub waiting: Option<RunWaiting>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunWaiting {
    pub on: Address,
    pub until: TimeMs,
}

impl RunWaiting {

    fn stated_by(record: &EventRecord) -> Option<RunWaiting> {
        let data = record.data().as_map();
        let on = data
            .get("on")
            .and_then(serde_json::Value::as_str)
            .and_then(|on| Address::parse(on).ok())?;
        let until = data
            .get("deadline_ms")
            .and_then(serde_json::Value::as_u64)
            .map(TimeMs::new)?;
        Some(RunWaiting { on, until })
    }
}

impl RunHot {
    fn first_seen(record: &EventRecord) -> RunHot {
        RunHot {
            phase: RunPhase::Active,
            last_seq: record.seq(),
            last_kind: record.kind(),
            who: record.who().to_owned(),
            addr: None,
            started: None,
            completion: None,
            pr: None,
            ask: None,
            task: None,
            goal: None,
            waiting: None,
        }
    }

    fn absorb(&mut self, record: &EventRecord) {
        let kind = record.kind();
        self.last_seq = record.seq();
        self.last_kind = kind;
        if kind == EventKind::RunFrozen {
            self.phase = RunPhase::Frozen;
        }
        if kind == EventKind::RunStarted {
            self.addr = record.addr().cloned();
            self.started = Some(record.t());
        }
        let stated = |field: &str| {
            record
                .data()
                .as_map()
                .get(field)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };
        if kind == EventKind::RunStarted {
            let named = |field: &str| stated(field).filter(|said| !said.is_empty());
            self.task = named("task");
            self.goal = named("goal");
        }
        if kind == EventKind::RunFrozen {
            self.completion = stated("completion");
        }
        if kind == EventKind::PrOpened {
            self.pr = stated("branch");
        }
        self.ask = (kind == EventKind::ApprovalRequested)
            .then(|| stated("action_desc"))
            .flatten();
        if kind == EventKind::SignalWaitStarted && self.phase == RunPhase::Active {
            self.waiting = RunWaiting::stated_by(record);
        }
        if kind == EventKind::SignalWaitEnded || kind == EventKind::RunFrozen {
            self.waiting = None;
        }
    }
}

pub const RECENT_FROZEN: usize = 32;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HotView {
    runs: BTreeMap<RunId, RunHot>,

    evicted: BTreeSet<RunId>,
}

impl HotView {
    pub fn new() -> HotView {
        HotView::default()
    }

    pub fn apply(&mut self, record: &EventRecord) -> Result<(), StorageError> {
        let run = record.run();

        if run == RunId::CITY || self.evicted.contains(&run) {
            return Ok(());
        }
        let kind = record.kind();
        match self.runs.entry(run) {
            Entry::Occupied(held) => {
                let hot = held.into_mut();
                if record.seq() > hot.last_seq {
                    hot.absorb(record);
                }
            }
            Entry::Vacant(slot) => slot.insert(RunHot::first_seen(record)).absorb(record),
        }
        if kind == EventKind::RunFrozen {
            self.evict_the_oldest_frozen_beyond_recent();
        }
        Ok(())
    }

    fn evict_the_oldest_frozen_beyond_recent(&mut self) {
        let frozen = || {
            self.runs
                .iter()
                .filter(|(_, hot)| hot.phase == RunPhase::Frozen)
        };
        if frozen().count() <= RECENT_FROZEN {
            return;
        }
        let Some(oldest) = frozen()
            .min_by_key(|(_, hot)| hot.last_seq)
            .map(|(run, _)| *run)
        else {
            return;
        };
        self.runs.remove(&oldest);
        self.evicted.insert(oldest);
    }

    pub fn runs(&self) -> impl Iterator<Item = (&RunId, &RunHot)> {
        self.runs.iter()
    }

    pub fn get(&self, run: &RunId) -> Option<&RunHot> {
        self.runs.get(run)
    }

    pub fn was_evicted(&self, run: &RunId) -> bool {
        self.evicted.contains(run)
    }

    pub fn active_count(&self) -> u64 {
        self.count_phase(RunPhase::Active)
    }

    pub fn frozen_count(&self) -> u64 {
        let evicted = u64::try_from(self.evicted.len()).unwrap_or(u64::MAX);
        self.count_phase(RunPhase::Frozen).saturating_add(evicted)
    }

    fn count_phase(&self, phase: RunPhase) -> u64 {
        u64::try_from(self.runs.values().filter(|h| h.phase == phase).count()).unwrap_or(u64::MAX)
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
