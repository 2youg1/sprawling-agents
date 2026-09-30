// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

//! What the rounds fold answers, asserted case by case: a server-side
//! fold that satisfies these answers what a client folding the same
//! session's records would.

use super::{opened_at, turns};
use wire::{B3Hash, EventDraft, EventKind, EventRecord, Outcome, Payload, RunId, Seq, TimeMs};

fn record(seq: u64, kind: EventKind, data: serde_json::Value) -> EventRecord {
    let map = data.as_object().expect("a payload is an object").clone();
    EventRecord::from_draft(
        EventDraft {
            run: RunId::from_bytes([7u8; 16]),
            t: TimeMs::new(seq),
            who: "lab/parser".to_owned(),
            addr: None,
            kind,
            data: Payload::new(map).expect("a payload"),
            ig: false,
        },
        Seq::new(seq),
        B3Hash::digest(b"prev"),
    )
}

fn called(seq: u64, id: &str, name: &str, path: &str) -> EventRecord {
    record(
        seq,
        EventKind::ToolCalled,
        serde_json::json!({ "id": id, "name": name, "args": { "path": path },
                            "subject": path }),
    )
}

fn answered(seq: u64, id: &str) -> EventRecord {
    record(
        seq,
        EventKind::ToolResult,
        serde_json::json!({ "tool_use_id": id, "name": "read", "result": { "lines": 412 } }),
    )
}

fn failed(seq: u64, id: &str) -> EventRecord {
    record(
        seq,
        EventKind::ToolResult,
        serde_json::json!({ "tool_use_id": id, "name": "exec", "error": { "code": 101 } }),
    )
}

fn asked(seq: u64) -> EventRecord {
    record(seq, EventKind::ModelCalled, serde_json::json!({}))
}

#[test]
fn a_turn_opens_when_the_model_is_asked_and_gathers_what_followed() {
    let events = [
        asked(1),
        called(2, "a", "read", "src/lex.rs"),
        answered(3, "a"),
    ];
    let folded = turns(&events);
    assert_eq!(folded.len(), 1);
    assert_eq!(folded[0].number, 1);
    assert_eq!(folded[0].opened, Seq::new(1));
    assert_eq!(folded[0].calls.len(), 1);
    assert_eq!(folded[0].calls[0].tool, "read");
    assert_eq!(folded[0].calls[0].subject.as_deref(), Some("src/lex.rs"));
    assert_eq!(folded[0].calls[0].outcome, Outcome::Answered);
}

#[test]
fn turns_are_numbered_in_the_order_they_opened() {
    let events = [
        asked(1),
        called(2, "a", "read", "x"),
        asked(3),
        called(4, "b", "edit", "y"),
    ];
    let folded = turns(&events);
    assert_eq!(folded.len(), 2);
    assert_eq!((folded[0].number, folded[1].number), (1, 2));
    assert_eq!(folded[1].calls[0].tool, "edit");
}

#[test]
fn an_answer_finds_its_own_call_and_not_the_nearest_one() {
    // Two calls go out before either answers, and the second answers
    // first. Pairing by position would mark the wrong one failed.
    let events = [
        asked(1),
        called(2, "a", "read", "x"),
        called(3, "b", "exec", "cargo test"),
        failed(4, "b"),
        answered(5, "a"),
    ];
    let folded = turns(&events);
    assert_eq!(folded[0].calls[0].outcome, Outcome::Answered, "read");
    assert_eq!(folded[0].calls[1].outcome, Outcome::Failed, "exec");
}

#[test]
fn a_call_carries_the_ledger_times_it_was_called_and_answered() {
    // The records' clock is their sequence number, so the call at 3 that
    // answers at 5 reads as 3..5, and the one never answered has no end.
    let events = [
        asked(1),
        called(3, "a", "read", "x"),
        called(4, "b", "exec", "cargo build"),
        answered(5, "a"),
    ];
    let times: Vec<_> = turns(&events)[0]
        .calls
        .iter()
        .map(|call| (call.called, call.answered))
        .collect();
    assert_eq!(
        times,
        [
            (TimeMs::new(3), Some(TimeMs::new(5))),
            (TimeMs::new(4), None)
        ]
    );
}

#[test]
fn a_turn_carries_the_model_its_model_called_named() {
    let events = [
        record(
            1,
            EventKind::ModelCalled,
            serde_json::json!({ "segments": [], "model": "big-1" }),
        ),
        record(
            2,
            EventKind::ModelCalled,
            serde_json::json!({ "segments": [], "model": "small-2" }),
        ),
        asked(3),
    ];
    let models: Vec<_> = turns(&events).into_iter().map(|turn| turn.model).collect();
    assert_eq!(
        models,
        [Some("big-1".to_owned()), Some("small-2".to_owned()), None]
    );
}

#[test]
fn a_call_still_running_says_so_rather_than_looking_finished() {
    let events = [asked(1), called(2, "a", "exec", "cargo build")];
    assert_eq!(turns(&events)[0].calls[0].outcome, Outcome::Waiting);
}

#[test]
fn an_answer_to_a_call_this_window_never_saw_is_dropped_not_guessed() {
    // The window is bounded, so its first rows can answer calls that
    // scrolled out. Attaching one to whatever call is nearest would
    // report an outcome that never happened.
    let events = [asked(1), called(2, "a", "read", "x"), answered(3, "gone")];
    let folded = turns(&events);
    assert_eq!(folded[0].calls.len(), 1);
    assert_eq!(folded[0].calls[0].outcome, Outcome::Waiting);
}

#[test]
fn work_before_the_first_turn_belongs_to_no_turn() {
    // A result arriving before any model call has no round to sit in,
    // and inventing turn zero would put the session's opening inside
    // a turn nobody took.
    let events = [answered(1, "a"), asked(2)];
    let folded = turns(&events);
    assert_eq!(folded.len(), 1);
    assert!(folded[0].calls.is_empty());
}

#[test]
fn a_call_whose_arguments_this_build_cannot_read_still_gets_a_row() {
    // Fail-open for a view: a reader one version behind must show the
    // call it cannot parse, not hide it.
    let odd = record(
        2,
        EventKind::ToolCalled,
        serde_json::json!({ "id": "a", "name": "future", "args": { "shape": 3 } }),
    );
    let events = [asked(1), odd];
    let folded = turns(&events);
    assert_eq!(folded[0].calls[0].tool, "future");
    assert_eq!(folded[0].calls[0].subject, None);
}

#[test]
fn the_fold_shows_the_carried_subject_not_a_reading_of_the_arguments() {
    // The subject is decided when the record is written; the fold reads
    // it. A fold that looked at `args` again would be the second answer
    // this field exists to remove.
    let events = [
        asked(1),
        record(
            2,
            EventKind::ToolCalled,
            serde_json::json!({ "id": "a", "name": "note",
                                "args": { "path": "src/lex.rs" },
                                "subject": "the carried one" }),
        ),
    ];
    assert_eq!(
        turns(&events)[0].calls[0].subject.as_deref(),
        Some("the carried one")
    );
}

#[test]
fn the_bytes_stay_addressable_because_every_call_carries_its_seq() {
    let events = [asked(1), called(9, "a", "read", "x")];
    assert_eq!(turns(&events)[0].calls[0].at, Seq::new(9));
}

/// The base a change list is addressed by is the session's first checkpoint,
/// not its latest one.
#[test]
fn the_change_base_is_the_first_checkpoint_of_the_session() {
    let checkpoint = |seq: u64, oid: &str| {
        record(
            seq,
            EventKind::CheckpointCommitted,
            serde_json::json!({ "oid": oid }),
        )
    };
    let first = "a".repeat(40);
    let second = "b".repeat(40);
    let events = [
        asked(1),
        checkpoint(2, &first),
        asked(3),
        checkpoint(4, &second),
    ];
    let folded = turns(&events);
    assert_eq!(
        opened_at(&folded).map(|oid| oid.to_string()),
        Some(first),
        "the tree as the work found it"
    );
}

/// The whole point of the card, asserted through the production door:
/// a page that asks `Query::Rounds` is handed exactly the fold of the
/// same records a client would compute for itself.
#[test]
fn asking_for_rounds_answers_the_fold_the_view_layer_ran() {
    use kernel::Ledger;
    let dir = tempfile::tempdir().unwrap();
    let report = crate::views::tests::founded(dir.path());
    let run = RunId::from_bytes([7u8; 16]);
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(9))
        .unwrap()
        .0;
    let drafts = [
        (EventKind::ModelCalled, serde_json::json!({})),
        (
            EventKind::ToolCalled,
            serde_json::json!({ "id": "a", "name": "read",
                                "args": { "path": "src/lex.rs" },
                                "subject": "src/lex.rs" }),
        ),
        (
            EventKind::ToolResult,
            serde_json::json!({ "tool_use_id": "a", "name": "read", "result": "412 lines" }),
        ),
    ];
    for (kind, data) in drafts {
        ledger
            .append(EventDraft {
                run,
                t: TimeMs::new(9),
                who: "lab/parser".to_owned(),
                addr: None,
                kind,
                data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
                ig: false,
            })
            .unwrap();
    }
    drop(ledger);

    let mut views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();
    let wire::Answer::Rounds(answer) = views.answer(&wire::Query::Rounds { run }) else {
        panic!("Rounds answers with rounds");
    };
    let records = views.ledger_ask().records_of(run);
    assert_eq!(answer.run, run);
    assert_eq!(
        answer.turns,
        turns(records.iter()),
        "the server's answer is the view layer's fold of the same records"
    );
    assert_eq!(answer.turns.len(), 1, "one model call, one round");
    assert_eq!(answer.turns[0].calls[0].outcome, Outcome::Answered);
    assert_eq!(
        answer.turns[0].calls[0].subject.as_deref(),
        Some("src/lex.rs")
    );
}

/// A conversation has a first line and a last line, and neither is a
/// turn: the task the person gave, and the word the run froze with.
#[test]
fn the_rounds_carry_how_the_session_opened_and_closed() {
    use kernel::Ledger;
    let dir = tempfile::tempdir().unwrap();
    let report = crate::views::tests::founded(dir.path());
    let run = RunId::from_bytes([8u8; 16]);
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(9))
        .unwrap()
        .0;
    let drafts = [
        (
            EventKind::RunStarted,
            // A whole locator, because `run_started` states one and
            // `kernel::event::record::RunStarted` parses it; "j" was
            // only ever readable while every key was read on its own.
            serde_json::json!({
                "task": "plan the week",
                "goal": "a roadmap",
                "job": "file:hall/mayor@0123456789abcdef0123456789abcdef01234567",
                "dispatched_by": "person",
            }),
            TimeMs::new(10),
        ),
        (
            EventKind::ModelCalled,
            serde_json::json!({}),
            TimeMs::new(11),
        ),
        (
            EventKind::RunFrozen,
            serde_json::json!({ "completion": "done", "evidence": [] }),
            TimeMs::new(12),
        ),
    ];
    for (kind, data, t) in drafts {
        ledger
            .append(EventDraft {
                run,
                t,
                who: "city".to_owned(),
                addr: Some(kernel::Address::parse("hall/mayor").unwrap()),
                kind,
                data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
                ig: false,
            })
            .unwrap();
    }
    drop(ledger);

    let mut views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();
    let wire::Answer::Rounds(answer) = views.answer(&wire::Query::Rounds { run }) else {
        panic!("Rounds answers with rounds");
    };
    assert_eq!(
        answer.opening,
        Some(wire::Opening {
            task: "plan the week".to_owned(),
            goal: "a roadmap".to_owned(),
            at: TimeMs::new(10),
            dispatched_by: Some(kernel::event::Who::Person),
        }),
        "the opening carries who dispatched the run, as run_started records it"
    );
    let closing = answer.closing.expect("the window held run_frozen");
    assert_eq!(closing.completion, "done");
    assert_eq!(closing.at, TimeMs::new(12));
}
