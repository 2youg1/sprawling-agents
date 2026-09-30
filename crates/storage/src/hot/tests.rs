// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
    // A checkpoint lands before the opening, and carries no address.
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

/// A city that ran thousands of times held a row for every run it
/// ever froze; only the active runs and the recent few stay, and a
/// late record on an evicted run is its tail, not a new run's checkpoint.
#[test]
fn frozen_runs_beyond_the_recent_few_leave_a_tombstone() {
    let mut view = HotView::new();
    let recent = u64::try_from(RECENT_FROZEN).unwrap();
    let run_of = |i: u64| RunId::from_bytes(u128::from(i + 1).to_be_bytes());
    for i in 0..=recent {
        view.apply(&record(run_of(i), 2 * i, EventKind::RunStarted))
            .unwrap();
        view.apply(&record(run_of(i), 2 * i + 1, EventKind::RunFrozen))
            .unwrap();
    }
    let tail = 2 * recent + 2;
    view.apply(&record(run_of(0), tail, EventKind::CheckpointCommitted))
        .unwrap();
    let held: Vec<RunId> = view.runs().map(|(run, _)| *run).collect();
    let newest: Vec<RunId> = (1..=recent).map(run_of).collect();
    assert_eq!(
        (held, view.active_count(), view.frozen_count()),
        (newest, 0, recent + 1)
    );
}

/// A record whose payload states one field: `said` is the field and its value.
fn stating(run: RunId, seq: u64, kind: EventKind, said: (&str, &str)) -> EventRecord {
    let (field, value) = said;
    let mut data = serde_json::Map::new();
    data.insert(
        field.to_owned(),
        serde_json::Value::String(value.to_owned()),
    );
    let draft = EventDraft {
        run,
        t: TimeMs::new(seq),
        who: "resident".to_owned(),
        addr: None,
        kind,
        data: Payload::new(data).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b""))
}

/// The results city reads a reloaded run's ending, its pull request
/// and what it waits for from `city_view` alone, so the fold keeps
/// all three; the ask lives only while the request is the last word.
#[test]
fn the_fold_keeps_the_ending_the_pull_request_and_the_ask() {
    let mut view = HotView::new();
    let run = RunId::from_bytes([5u8; 16]);
    view.apply(&record(run, 0, EventKind::RunStarted)).unwrap();
    view.apply(&stating(
        run,
        1,
        EventKind::ApprovalRequested,
        ("action_desc", "publish"),
    ))
    .unwrap();
    assert_eq!(view.get(&run).unwrap().ask.as_deref(), Some("publish"));
    view.apply(&record(run, 2, EventKind::ToolCalled)).unwrap();
    assert_eq!(view.get(&run).unwrap().ask, None, "the run moved on");
    view.apply(&stating(
        run,
        3,
        EventKind::PrOpened,
        ("branch", "gate-btree"),
    ))
    .unwrap();
    view.apply(&stating(
        run,
        4,
        EventKind::RunFrozen,
        ("completion", "done"),
    ))
    .unwrap();
    let hot = view.get(&run).unwrap();
    assert_eq!(
        (hot.completion.as_deref(), hot.pr.as_deref()),
        (Some("done"), Some("gate-btree"))
    );
}

/// The run board titles a reloaded run by what the person asked, so
/// the fold keeps the opening's task and goal; an empty one is no name.
#[test]
fn the_fold_keeps_the_task_and_the_goal_the_run_was_started_with() {
    let mut view = HotView::new();
    let run = RunId::from_bytes([6u8; 16]);
    let mut data = serde_json::Map::new();
    data.insert("task".to_owned(), serde_json::json!("draft the plan"));
    data.insert("goal".to_owned(), serde_json::json!(""));
    let draft = EventDraft {
        run,
        t: TimeMs::new(0),
        who: "city".to_owned(),
        addr: None,
        kind: EventKind::RunStarted,
        data: Payload::new(data).unwrap(),
        ig: false,
    };
    view.apply(&EventRecord::from_draft(
        draft,
        Seq::new(0),
        B3Hash::digest(b""),
    ))
    .unwrap();
    view.apply(&record(run, 1, EventKind::ToolCalled)).unwrap();
    let hot = view.get(&run).unwrap();
    assert_eq!(
        (hot.task.as_deref(), hot.goal.as_deref()),
        (Some("draft the plan"), None)
    );
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
