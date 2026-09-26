// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The in-memory hot view: what the interface asks
//! for constantly — which Runs exist, which phase each is in, how far
//! each has got — answered without touching the disk.
//!
//! It is a fold over the ledger, nothing more. Feeding the same record
//! twice changes nothing, because seq only moves forward; that is what
//! lets a caller replay from any point without first proving where it
//! left off.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use kernel::{Address, EventKind, EventRecord, RunId, Seq, TimeMs};

use crate::error::MemoryError;

/// A Run's phase as the hot view sees it. Freezing is terminal here:
/// the ledger may keep appending to a frozen Run's history, but the
/// phase never travels backwards.
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
    /// The room this run works in, as its `run_started` record named
    /// it. `None` until that record is seen: a fence can land before
    /// the opening, and a window may hold only a tail.
    pub addr: Option<Address>,
    /// When the run began, from the same record.
    pub started: Option<TimeMs>,
    /// How the run ended, from its `run_frozen` record's `completion`.
    pub completion: Option<String>,
    /// The branch of the last pull request the run opened: the city's
    /// pull requests are named by their branch and carry no number.
    pub pr: Option<String>,
    /// What the run waits for the person to allow. Present exactly while
    /// the run's last record is an `approval_requested`, so it cannot say
    /// a run waits when `last_kind` says it moved on.
    pub ask: Option<String>,
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
        }
    }

    /// Folds one record that is newer than everything this run has seen.
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
        if kind == EventKind::RunFrozen {
            self.completion = stated("completion");
        }
        if kind == EventKind::PrOpened {
            self.pr = stated("branch");
        }
        self.ask = (kind == EventKind::ApprovalRequested)
            .then(|| stated("action_desc"))
            .flatten();
    }
}

/// How many frozen runs the hot view holds besides every active one,
/// and so how many a city view carries. A bound on the size of an
/// answer on the wire, not a machine reading, so it is a constant
/// (memory-SPEC section 8-5).
pub const RECENT_FROZEN: usize = 32;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct HotView {
    runs: BTreeMap<RunId, RunHot>,
    /// Frozen runs pushed out of `runs`, by id alone. Freezing is
    /// terminal, so a later record on one of these is the tail of an old
    /// run, never the fence of a new one.
    evicted: BTreeSet<RunId>,
}

impl HotView {
    pub fn new() -> HotView {
        HotView::default()
    }

    /// Folds one record in. Records at or below a Run's last seq are
    /// ignored, so re-feeding a segment is free of consequence.
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), MemoryError> {
        let run = record.run();
        // A city-level record belongs to the city, not to a run: raising a
        // building and the genesis record both carry the nil id, and the
        // Ledger says so (`kernel::event`: "RunId::CITY (nil) marks
        // city-level records"). Admitting one here invented a run nobody
        // started, and the count it fed said a city was working the
        // moment it existed.
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

    /// Keeps at most [`RECENT_FROZEN`] frozen runs, tombstoning the one
    /// with the lowest `last_seq`. One freeze adds one frozen run, so one
    /// eviction restores the bound; the scan is O(active + RECENT_FROZEN).
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

    /// Every active run and the [`RECENT_FROZEN`] frozen runs with the
    /// latest `last_seq`: what a city view carries. Iteration is in RunId
    /// order — the same order on every process, so a rendered list never
    /// reshuffles between restarts.
    pub fn runs(&self) -> impl Iterator<Item = (&RunId, &RunHot)> {
        self.runs.iter()
    }

    pub fn get(&self, run: &RunId) -> Option<&RunHot> {
        self.runs.get(run)
    }

    /// Whether `run` froze and was pushed out of the view. Its records
    /// are still in the Ledger; a reader that needs them goes there.
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
