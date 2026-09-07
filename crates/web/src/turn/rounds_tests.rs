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

//! The rounds through the production door.

use super::super::reading::Outcome;
use super::super::rounds::turns;
use channels::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};

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
        serde_json::json!({ "id": id, "name": name, "args": { "path": path } }),
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
    // Fail-open for a view: a client one version behind must show the
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
fn a_tool_with_no_preferred_key_is_still_named_by_what_it_acted_on() {
    let events = [
        asked(1),
        record(
            2,
            EventKind::ToolCalled,
            serde_json::json!({ "id": "a", "name": "note", "args": { "body": "ship it" } }),
        ),
    ];
    assert_eq!(
        turns(&events)[0].calls[0].subject.as_deref(),
        Some("ship it")
    );
}

#[test]
fn the_bytes_stay_addressable_because_every_call_carries_its_seq() {
    let events = [asked(1), called(9, "a", "read", "x")];
    assert_eq!(turns(&events)[0].calls[0].at, Seq::new(9));
}
