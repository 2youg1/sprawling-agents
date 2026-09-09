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

use std::collections::BTreeMap;

use kernel::{Address, EventKind, EventRecord, RunId, Seq, TimeMs};

use crate::error::MemoryError;

/// A Run's phase as the hot view sees it. Freezing is terminal here:
/// the ledger may keep appending to a frozen Run's history, but the
/// phase never travels backwards.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Active,
    Frozen,
}

#[derive(Debug, Clone)]
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
}

#[derive(Default)]
pub struct HotView {
    runs: BTreeMap<RunId, RunHot>,
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
        if run == RunId::CITY {
            return Ok(());
        }
        let seq = record.seq();
        let kind = record.kind();
        match self.runs.get_mut(&run) {
            Some(hot) => {
                if seq <= hot.last_seq {
                    return Ok(());
                }
                hot.last_seq = seq;
                hot.last_kind = kind;
                if kind == EventKind::RunFrozen {
                    hot.phase = RunPhase::Frozen;
                }
                if kind == EventKind::RunStarted {
                    hot.addr = record.addr().cloned();
                    hot.started = Some(record.t());
                }
            }
            None => {
                let phase = if kind == EventKind::RunFrozen {
                    RunPhase::Frozen
                } else {
                    RunPhase::Active
                };
                let opened = kind == EventKind::RunStarted;
                self.runs.insert(
                    run,
                    RunHot {
                        phase,
                        last_seq: seq,
                        last_kind: kind,
                        who: record.who().to_owned(),
                        addr: opened.then(|| record.addr().cloned()).flatten(),
                        started: opened.then(|| record.t()),
                    },
                );
            }
        }
        Ok(())
    }

    /// Iteration is in RunId order — the same order on every process,
    /// so a rendered list never reshuffles between restarts.
    pub fn runs(&self) -> impl Iterator<Item = (&RunId, &RunHot)> {
        self.runs.iter()
    }

    pub fn get(&self, run: &RunId) -> Option<&RunHot> {
        self.runs.get(run)
    }

    pub fn active_count(&self) -> u64 {
        self.count_phase(RunPhase::Active)
    }

    pub fn frozen_count(&self) -> u64 {
        self.count_phase(RunPhase::Frozen)
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
mod tests {
    use super::*;
    use kernel::{B3Hash, EventDraft, Payload, TimeMs};

    fn record(run: RunId, seq: u64, kind: EventKind) -> EventRecord {
        let draft = EventDraft {
            run,
            t: TimeMs::new(seq),
            who: "resident".to_owned(),
            addr: None,
            kind,
            data: Payload::new(serde_json::Map::new()).unwrap(),
            ig: false,
        };
        EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b""))
    }

    #[test]
    fn the_fold_tracks_phase_and_progress_per_run() {
        let mut view = HotView::new();
        let one = RunId::from_bytes([1u8; 16]);
        let two = RunId::from_bytes([2u8; 16]);
        view.apply(&record(one, 0, EventKind::RunStarted)).unwrap();
        view.apply(&record(two, 1, EventKind::RunStarted)).unwrap();
        view.apply(&record(one, 2, EventKind::ToolCalled)).unwrap();
        assert_eq!(view.active_count(), 2);
        assert_eq!(view.frozen_count(), 0);
        assert_eq!(view.get(&one).unwrap().last_seq, Seq::new(2));
        assert_eq!(view.get(&one).unwrap().last_kind, EventKind::ToolCalled);
        assert_eq!(view.get(&one).unwrap().who, "resident");

        view.apply(&record(one, 3, EventKind::RunFrozen)).unwrap();
        assert_eq!(view.active_count(), 1);
        assert_eq!(view.frozen_count(), 1);
        assert_eq!(view.get(&one).unwrap().phase, RunPhase::Frozen);
    }

    /// A page asking `city_view` needs to know which room a run works
    /// in and when it began, and `who` says neither: it is the author
    /// of the first record, which is always the city. Both facts are on
    /// the `run_started` record and nowhere else, so the fold takes
    /// them there and leaves them alone afterwards.
    #[test]
    fn the_room_and_the_start_come_from_run_started_only() {
        let mut view = HotView::new();
        let run = RunId::from_bytes([4u8; 16]);
        // A fence lands before the opening, and carries no address.
        view.apply(&record(run, 0, EventKind::CheckpointCommitted))
            .unwrap();
        assert_eq!(view.get(&run).unwrap().addr, None);
        assert_eq!(view.get(&run).unwrap().started, None);

        let room = kernel::Address::parse("hall/mayor").unwrap();
        let opened = EventDraft {
            run,
            t: TimeMs::new(1_700),
            who: "city".to_owned(),
            addr: Some(room.clone()),
            kind: EventKind::RunStarted,
            data: Payload::new(serde_json::Map::new()).unwrap(),
            ig: false,
        };
        view.apply(&EventRecord::from_draft(
            opened,
            Seq::new(1),
            B3Hash::digest(b""),
        ))
        .unwrap();
        assert_eq!(view.get(&run).unwrap().addr, Some(room.clone()));
        assert_eq!(view.get(&run).unwrap().started, Some(TimeMs::new(1_700)));

        // Later records, with or without an address, do not move them.
        let later = EventDraft {
            run,
            t: TimeMs::new(1_900),
            who: "resident".to_owned(),
            addr: Some(kernel::Address::parse("lab").unwrap()),
            kind: EventKind::ToolCalled,
            data: Payload::new(serde_json::Map::new()).unwrap(),
            ig: false,
        };
        view.apply(&EventRecord::from_draft(
            later,
            Seq::new(2),
            B3Hash::digest(b""),
        ))
        .unwrap();
        assert_eq!(view.get(&run).unwrap().addr, Some(room));
        assert_eq!(view.get(&run).unwrap().started, Some(TimeMs::new(1_700)));
    }

    #[test]
    fn a_city_level_record_is_not_a_run() {
        // `RunId::CITY` is the nil id that marks a record belonging to the
        // city rather than to any run - raising a building, the genesis
        // record. Folding one into the run table made a city that had
        // never been dispatched to report one run in flight, which the
        // interface then showed on its city page while its overview,
        // folding the same stream client-side, showed none. Two answers to
        // one question, and the wrong one was the server's.
        let mut view = HotView::new();
        view.apply(&record(RunId::CITY, 0, EventKind::CityInitialized))
            .unwrap();
        view.apply(&record(RunId::CITY, 1, EventKind::BuildingCreated))
            .unwrap();
        assert_eq!(view.active_count(), 0, "a city is not working by existing");
        assert_eq!(view.runs().count(), 0);
        assert!(view.get(&RunId::CITY).is_none());

        // And a real run in the same city still counts.
        let one = RunId::from_bytes([1u8; 16]);
        view.apply(&record(one, 2, EventKind::RunStarted)).unwrap();
        assert_eq!(view.active_count(), 1);
    }

    #[test]
    fn replaying_the_same_records_changes_nothing() {
        let mut view = HotView::new();
        let run = RunId::from_bytes([3u8; 16]);
        let records = [
            record(run, 0, EventKind::RunStarted),
            record(run, 1, EventKind::ToolCalled),
            record(run, 2, EventKind::RunFrozen),
        ];
        for r in &records {
            view.apply(r).unwrap();
        }
        let before = view.get(&run).unwrap().clone();
        // Feed the whole segment again, and an out-of-order straggler.
        for r in &records {
            view.apply(r).unwrap();
        }
        view.apply(&record(run, 1, EventKind::ToolCalled)).unwrap();
        let after = view.get(&run).unwrap();
        assert_eq!(before.last_seq, after.last_seq);
        assert_eq!(before.last_kind, after.last_kind);
        assert_eq!(after.phase, RunPhase::Frozen, "freezing does not undo");
    }

    #[test]
    fn iteration_is_runid_ordered_not_insertion_ordered() {
        let mut view = HotView::new();
        // Fed in descending id order; iteration must still ascend.
        let mut ids: Vec<RunId> = (1..=8u8)
            .rev()
            .map(|b| RunId::from_bytes([b; 16]))
            .collect();
        for (i, run) in ids.iter().enumerate() {
            view.apply(&record(
                *run,
                u64::try_from(i).unwrap(),
                EventKind::RunStarted,
            ))
            .unwrap();
        }
        ids.sort();
        let seen: Vec<RunId> = view.runs().map(|(id, _)| *id).collect();
        assert_eq!(seen, ids);
    }
}
