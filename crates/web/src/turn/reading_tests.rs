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

//! The reading through the production door.

use super::super::reading::Note;
use super::super::reading::OUTPUT_LINES;
use super::super::rounds::turns;
use channels::{
    AxCode, AxError, B3Hash, EventDraft, EventKind, EventRecord, GitOid, Payload, RunId, Seq,
    TimeMs, Tokens, UsdMicros,
};

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

fn asked(seq: u64) -> EventRecord {
    record(seq, EventKind::ModelCalled, serde_json::json!({}))
}

/// The payload `runtime::turn` writes when the model answers. Five
/// fields, and this fold used to read none of them.
fn answered_model(seq: u64, text: &str) -> EventRecord {
    record(
        seq,
        EventKind::ModelReturned,
        serde_json::json!({
            "message": {
                "role": "assistant",
                "content": [
                    { "kind": "thinking", "thinking": "weighing it", "signature": "sig" },
                    { "kind": "text", "text": text }
                ]
            },
            "calls": 0,
            "usage": {
                "input_tokens": 1200,
                "output_tokens": 340,
                "cache_read_tokens": 800,
                "cache_write_tokens": 0
            },
            "stop": "end_turn",
            "billed_usd_micros": 3340
        }),
    )
}

#[test]
fn a_turn_says_what_the_model_said_and_what_it_cost() {
    let folded = turns(&[asked(1), answered_model(2, "I read the lexer.")]);
    assert_eq!(folded[0].said.as_deref(), Some("I read the lexer."));
    assert_eq!(folded[0].spent, Some(UsdMicros::new(3340)));
    assert_eq!(folded[0].stopped.as_deref(), Some("end_turn"));
    let used = folded[0].used.expect("usage is on the wire");
    assert_eq!(used.input, Tokens::new(1200));
    assert_eq!(used.output, Tokens::new(340));
    assert_eq!(used.cached, Tokens::new(800));
}

/// Thinking blocks are carried end to end so the provider can verify
/// their signature. Rendering them would be this page publishing
/// something it was only ever asked to relay.
#[test]
fn what_the_model_was_thinking_is_relayed_not_rendered() {
    let folded = turns(&[asked(1), answered_model(2, "done")]);
    let said = folded[0].said.clone().unwrap_or_default();
    assert!(!said.contains("weighing it"), "{said}");
}

/// A client one version behind must still show the turn.
#[test]
fn a_model_return_this_build_cannot_read_still_leaves_the_turn_standing() {
    let odd = record(
        2,
        EventKind::ModelReturned,
        serde_json::json!({ "shape": 3 }),
    );
    let folded = turns(&[asked(1), odd]);
    assert_eq!(folded.len(), 1);
    assert_eq!(folded[0].said, None);
    assert_eq!(folded[0].spent, None);
    assert_eq!(folded[0].used, None);
}

#[test]
fn a_call_that_answered_says_what_it_said() {
    let events = [
        asked(1),
        called(2, "a", "exec", "cargo test"),
        record(
            3,
            EventKind::ToolResult,
            serde_json::json!({ "tool_use_id": "a", "name": "exec",
                                "result": "test result: ok. 1162 passed" }),
        ),
    ];
    let folded = turns(&events);
    let output = folded[0].calls[0]
        .output
        .clone()
        .expect("an answered call says what it said");
    assert!(output.head.contains("1162 passed"), "{}", output.head);
    assert_eq!(output.cut, 0);
}

/// The disclosure has a bound, and the bound reports itself. A wave of
/// output that silently became the page is the dump section 8-47
/// refuses.
#[test]
fn a_long_output_is_cut_and_says_how_much_it_cut() {
    let long: String = (0..40)
        .map(|n| format!("line {n}\n"))
        .collect::<Vec<String>>()
        .join("");
    let events = [
        asked(1),
        called(2, "a", "exec", "cargo build"),
        record(
            3,
            EventKind::ToolResult,
            serde_json::json!({ "tool_use_id": "a", "name": "exec", "result": long }),
        ),
    ];
    let output = turns(&events)[0].calls[0]
        .output
        .clone()
        .expect("an answered call says what it said");
    assert_eq!(output.head.lines().count(), OUTPUT_LINES);
    assert_eq!(output.cut, 40 - OUTPUT_LINES);
}

/// The headline case: a three-part refusal used to render as one grey
/// line indistinguishable from a successful read.
#[test]
fn a_door_that_refused_lands_in_the_turn_whole() {
    let refusal = AxError::failure(
        AxCode::OutsideWriteDomain,
        "write",
        "crates/kernel/src/gate.rs",
    )
    .with_recovery("write under lab/ instead");
    let payload = serde_json::to_value(&refusal).expect("an error serialises");
    let events = [asked(1), record(2, EventKind::GateDenied, payload)];
    let folded = turns(&events);
    match folded[0].notes.first() {
        Some(Note::Refused { error, at }) => {
            assert_eq!(*error, refusal, "the error travels whole");
            assert_eq!(*at, Seq::new(2));
        }
        other => panic!("a refusal is a note on the turn, got {other:?}"),
    }
}

#[test]
fn a_checkpoint_inside_a_turn_names_the_commit_it_made() {
    let spelled = "3f9a1c00112233445566778899aabbccddeeff00";
    let events = [
        asked(1),
        record(
            2,
            EventKind::CheckpointCommitted,
            serde_json::json!({ "oid": spelled, "scope": "lab", "files": [] }),
        ),
    ];
    assert_eq!(
        turns(&events)[0].notes,
        vec![Note::Fenced {
            oid: GitOid::parse(spelled).expect("forty hex digits"),
            at: Seq::new(2)
        }]
    );
}

/// The oid is what a change list is addressed by, so a spelling this
/// build cannot parse leaves no row: a checkpoint nothing can be
/// asked about is worse than a checkpoint that is not shown, because
/// the first one looks like a working control.
#[test]
fn a_checkpoint_whose_oid_will_not_parse_leaves_no_row_to_click() {
    let events = [
        asked(1),
        record(
            2,
            EventKind::CheckpointCommitted,
            serde_json::json!({ "oid": "3f9a1c", "scope": "lab", "files": [] }),
        ),
    ];
    assert!(turns(&events)[0].notes.is_empty());
}

#[test]
fn a_word_from_a_person_lands_in_the_turn_it_reached() {
    let events = [
        asked(1),
        record(
            2,
            EventKind::SteerReceived,
            serde_json::json!({ "source": "user", "said": "ignore the cache" }),
        ),
    ];
    // `runtime::turn` writes `text`, not `said`: the fold reads the
    // field the producer writes, and an unknown shape still leaves a
    // note rather than dropping the fact that somebody spoke.
    match turns(&events)[0].notes.first() {
        Some(Note::Arrived { from, at, .. }) => {
            assert_eq!(from, "user");
            assert_eq!(*at, Seq::new(2));
        }
        other => panic!("a steer is a note on the turn, got {other:?}"),
    }
}

#[test]
fn a_turn_waiting_on_a_person_says_so_without_copying_the_queue() {
    let events = [
        asked(1),
        record(2, EventKind::ApprovalRequested, serde_json::json!({})),
    ];
    assert_eq!(
        turns(&events)[0].notes,
        vec![Note::Waiting { at: Seq::new(2) }]
    );
}

/// A note is only a note when it belongs to a turn. The session's own
/// opening is not a round somebody took.
#[test]
fn a_note_before_the_first_turn_belongs_to_no_turn() {
    let events = [
        record(
            1,
            EventKind::CheckpointCommitted,
            serde_json::json!({ "oid": "8c22de", "scope": "lab", "files": [] }),
        ),
        asked(2),
    ];
    let folded = turns(&events);
    assert_eq!(folded.len(), 1);
    assert!(folded[0].notes.is_empty());
}

/// The event stream keeps its own shape: a kind that changed neither
/// what the turn did nor what it waits on is not a note.
#[test]
fn an_event_that_changed_nothing_about_this_turn_is_not_a_note() {
    let events = [
        asked(1),
        record(2, EventKind::GateChecked, serde_json::json!({})),
        record(3, EventKind::PromptAssembled, serde_json::json!({})),
    ];
    assert!(turns(&events)[0].notes.is_empty());
}
