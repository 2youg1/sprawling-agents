// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! The fold, exercised through its production door.

use channels::{Address, EventKind, EventRecord, RunId, Seq};

use super::snapshot::Backfill;
use crate::app::snapshot::{Snapshot, rebuild};
use crate::phase::Phase;

/// One event, as a stream would deliver it.
#[cfg(test)]
pub(crate) fn record(seq: u64, kind: EventKind, run: [u8; 16]) -> EventRecord {
    EventRecord::from_draft(
        kernel_draft(kind, run),
        Seq::new(seq),
        channels::B3Hash::digest(b"prev"),
    )
}

/// The draft behind [`record`], separated because two tests want to set
/// a payload on it before it is sealed into a record.
#[cfg(test)]
fn kernel_draft(kind: EventKind, run: [u8; 16]) -> channels::EventDraft {
    channels::EventDraft {
        run: RunId::from_bytes(run),
        t: channels::TimeMs::new(1_000),
        who: "test".to_owned(),
        addr: None,
        kind,
        data: channels::Payload::empty(),
        ig: false,
    }
}

/// A snapshot holding the named sessions, folded from real records.
///
/// The production door and nothing beside it: every row here arrives
/// through [`Snapshot::apply`], so a test that seats a session is also
/// exercising the fold that seats one in a browser. A setter that wrote
/// into `runs` directly would let these tests pass while the fold was
/// broken, which is the failure they exist to catch.
#[cfg(test)]
pub(crate) fn seated(rows: &[(Option<&str>, Phase, u64)]) -> Snapshot {
    let mut snapshot = Snapshot::new();
    for (index, (addr, phase, seq)) in rows.iter().enumerate() {
        let mut run = [0u8; 16];
        // The index is what makes two rows two runs. Truncation cannot
        // happen below 256 rows and would only collide two fixtures.
        run[0] = u8::try_from(index).unwrap_or(u8::MAX);
        let id = RunId::from_bytes(run);
        snapshot.apply(&started(id, *addr, *seq));
        for record in ending(id, *phase, seq.saturating_add(1)) {
            snapshot.apply(&record);
        }
    }
    snapshot
}

/// A `model_returned` record carrying one text block, through the same
/// shape `runtime::turn` writes.
#[cfg(test)]
pub(crate) fn returned_for_test(run: RunId, said: &str, seq: u64) -> EventRecord {
    let mut data = serde_json::Map::new();
    data.insert(
        "message".to_owned(),
        serde_json::json!({ "content": [{ "kind": "text", "text": said }] }),
    );
    EventRecord::from_draft(
        channels::EventDraft {
            run,
            t: channels::TimeMs::new(1_000),
            who: "test".to_owned(),
            addr: None,
            kind: EventKind::ModelReturned,
            data: channels::Payload::new(data).unwrap_or_else(|_| channels::Payload::empty()),
            ig: false,
        },
        Seq::new(seq),
        channels::B3Hash::digest(b"prev"),
    )
}

#[cfg(test)]
fn started(run: RunId, addr: Option<&str>, seq: u64) -> EventRecord {
    EventRecord::from_draft(
        channels::EventDraft {
            run,
            t: channels::TimeMs::new(1_000),
            who: "test".to_owned(),
            addr: addr.and_then(|raw| Address::parse(raw).ok()),
            kind: EventKind::RunStarted,
            data: channels::Payload::empty(),
            ig: false,
        },
        Seq::new(seq),
        channels::B3Hash::digest(b"prev"),
    )
}

/// The records that put a run into the phase named, through the same
/// arms a live stream would take.
#[cfg(test)]
fn ending(run: RunId, phase: Phase, seq: u64) -> Vec<EventRecord> {
    let froze = |completion: &str| {
        let mut data = serde_json::Map::new();
        data.insert(
            "completion".to_owned(),
            serde_json::Value::String(completion.to_owned()),
        );
        vec![EventRecord::from_draft(
            channels::EventDraft {
                run,
                t: channels::TimeMs::new(1_000),
                who: "test".to_owned(),
                addr: None,
                kind: EventKind::RunFrozen,
                data: channels::Payload::new(data).unwrap_or_else(|_| channels::Payload::empty()),
                ig: false,
            },
            Seq::new(seq),
            channels::B3Hash::digest(b"prev"),
        )]
    };
    match phase {
        Phase::Running => Vec::new(),
        Phase::Frozen => froze("done"),
        Phase::Cancelled => froze("cancelled"),
        Phase::Waiting | Phase::Halted => {
            let kind = if phase == Phase::Waiting {
                EventKind::ApprovalRequested
            } else {
                EventKind::CityHalted
            };
            vec![EventRecord::from_draft(
                channels::EventDraft {
                    run,
                    t: channels::TimeMs::new(1_000),
                    who: "test".to_owned(),
                    addr: None,
                    kind,
                    data: channels::Payload::empty(),
                    ig: false,
                },
                Seq::new(seq),
                channels::B3Hash::digest(b"prev"),
            )]
        }
    }
}

#[test]
fn folding_the_same_stream_twice_reaches_the_same_snapshot() {
    // The snapshot is a projection, not a history: discard it, fold
    // again, land on the same value (ARCHITECTURE section 1).
    let stream = [
        record(1, EventKind::RunStarted, [1u8; 16]),
        record(2, EventKind::ToolResult, [1u8; 16]),
        record(3, EventKind::ApprovalRequested, [1u8; 16]),
        record(4, EventKind::ApprovalResolved, [1u8; 16]),
    ];
    let once = rebuild(stream.iter());
    let twice = rebuild(stream.iter());
    assert_eq!(once, twice);
}

#[test]
fn redelivering_a_frame_after_a_reconnect_changes_nothing() {
    let mut snapshot = Snapshot::new();
    let started = record(1, EventKind::RunStarted, [2u8; 16]);
    let stepped = record(2, EventKind::ToolResult, [2u8; 16]);
    assert!(snapshot.apply(&started));
    assert!(snapshot.apply(&stepped));
    let after_first_pass = snapshot.clone();

    // A reconnect resumes a little early rather than computing an exact
    // cut; the overlap must be free.
    assert!(!snapshot.apply(&started));
    assert!(!snapshot.apply(&stepped));
    assert_eq!(snapshot, after_first_pass);
    assert_eq!(
        snapshot
            .run(&RunId::from_bytes([2u8; 16]))
            .unwrap()
            .steps_done,
        1
    );
}

#[test]
fn halting_the_city_marks_every_run_not_just_the_one_that_carried_it() {
    let mut snapshot = Snapshot::new();
    snapshot.apply(&record(1, EventKind::RunStarted, [3u8; 16]));
    snapshot.apply(&record(2, EventKind::RunStarted, [4u8; 16]));
    snapshot.apply(&record(3, EventKind::CityHalted, [3u8; 16]));
    assert!(snapshot.is_halted());
    for (_, row) in snapshot.runs() {
        assert_eq!(row.phase, Phase::Halted);
    }
}

#[test]
fn a_pending_count_never_goes_below_nothing() {
    // A client that joined mid-stream can see a resolution whose request
    // it never saw. Saturating is the honest answer; wrapping would
    // print four billion pending approvals.
    let mut snapshot = Snapshot::new();
    snapshot.apply(&record(1, EventKind::ApprovalResolved, [5u8; 16]));
    assert_eq!(snapshot.approvals_pending(), 0);
}

/// A tab opened over a city that has been running for a month used
/// to show an empty one: the server broadcasts what happens next and
/// never what happened.
#[test]
fn a_page_folds_the_history_it_was_not_connected_for() {
    let mut snapshot = Snapshot::new();
    let history = [
        record(1, EventKind::RunStarted, [1u8; 16]),
        record(2, EventKind::ModelReturned, [1u8; 16]),
    ];
    assert_eq!(snapshot.backfill(&history), Backfill::Folded(2));
    assert_eq!(snapshot.runs().count(), 1);
    assert_eq!(snapshot.resume_from(), Some(Seq::new(2)));

    // And the live stream continues from there rather than being
    // refused as a duplicate.
    assert!(snapshot.apply(&record(3, EventKind::RunFrozen, [1u8; 16])));
}

/// The fold is forward only, so replaying older records over newer
/// ones would put a finished run back on screen as running. A page
/// that has already folded live events is not the empty one this
/// exists to fix, so it says so instead of guessing.
#[test]
fn a_page_that_is_already_live_refuses_to_be_backfilled() {
    let mut snapshot = Snapshot::new();
    snapshot.apply(&record(9, EventKind::RunFrozen, [1u8; 16]));
    assert_eq!(
        snapshot.backfill(&[record(1, EventKind::RunStarted, [1u8; 16])]),
        Backfill::AlreadyLive
    );
    assert_eq!(snapshot.resume_from(), Some(Seq::new(9)));
}

#[test]
fn an_unmodelled_event_kind_advances_the_cursor_without_inventing_state() {
    let mut snapshot = Snapshot::new();
    assert!(snapshot.apply(&record(9, EventKind::PromptAssembled, [6u8; 16])));
    assert_eq!(snapshot.resume_from(), Some(Seq::new(9)));
    assert_eq!(snapshot.runs().count(), 0);
}
