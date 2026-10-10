// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! A run of the chosen room read record by record, as the CLI shows it.

use super::super::local_time;
use super::super::stream::Room;
use super::helpers::room;
use console_ffi::part::{Ending, Outcome, Verdict};
use console_ffi::scene::{Entry, TimeOfDay};
use kernel::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};

/// One record of the chosen room, `t` seconds after 01:00 UTC: the
/// run's start addressed to the room, as the city writes it, and every
/// later line written by the resident with no address, as a run does.
fn record(seconds: u64, kind: EventKind, data: serde_json::Value) -> EventRecord {
    let map = data.as_object().unwrap().clone();
    let started = kind == EventKind::RunStarted;
    EventRecord::from_draft(
        EventDraft {
            run: RunId::from_bytes([7u8; 16]),
            t: TimeMs::new((3_600 + seconds) * 1_000),
            who: if started { "city" } else { "lab/room1" }.to_owned(),
            addr: started.then(room),
            kind,
            data: Payload::new(map).unwrap(),
            ig: false,
        },
        Seq::new(seconds + 1),
        B3Hash::digest(b"prev"),
    )
}

fn at(seconds: u32) -> Option<TimeOfDay> {
    TimeOfDay::from_seconds(3_600 + seconds)
}

fn read(room_state: &mut Room, records: &[EventRecord]) -> Vec<Entry> {
    records
        .iter()
        .flat_map(|record| room_state.read(record, &room(), local_time::utc))
        .collect()
}

#[test]
fn a_run_reads_as_head_calls_reply_and_ending() {
    let records = [
        record(
            0,
            EventKind::RunStarted,
            serde_json::json!({ "task": "t", "effort": "high",
                "policy": { "mode": "work", "write": "create", "admit": "tested", "landing": "experiment" } }),
        ),
        record(
            2,
            EventKind::ModelCalled,
            serde_json::json!({ "segments": [], "model": "m-1" }),
        ),
        record(
            3,
            EventKind::ToolCalled,
            serde_json::json!({ "id": "a", "name": "read", "args": {}, "subject": "src/lex.rs" }),
        ),
        record(
            4,
            EventKind::ToolResult,
            serde_json::json!({ "tool_use_id": "a", "name": "read", "result": { "lines": 412 }, "took_us": 842 }),
        ),
        record(
            5,
            EventKind::ModelCalled,
            serde_json::json!({ "segments": [], "model": "m-1" }),
        ),
        record(
            6,
            EventKind::ToolCalled,
            serde_json::json!({ "id": "b", "name": "exec", "args": {}, "subject": "cargo test" }),
        ),
        record(
            9,
            EventKind::ToolResult,
            serde_json::json!({ "tool_use_id": "b", "name": "exec", "error": { "code": 101 } }),
        ),
        record(
            10,
            EventKind::ModelReturned,
            serde_json::json!({ "message": { "role": "assistant", "content": [
                { "kind": "thinking", "thinking": "weighing it", "signature": "sig" },
                { "kind": "text", "text": "The lexer drops the last token." }
            ] }, "calls": [] }),
        ),
        record(
            12,
            EventKind::RunFrozen,
            serde_json::json!({ "completion": "done" }),
        ),
    ];
    let mut seen = Room::default();
    let entries = read(&mut seen, &records);
    let wanted = vec![
        Entry::Head {
            at: at(2),
            resident: "room1".to_owned(),
            facts: vec!["m-1".to_owned(), "work".to_owned(), "high".to_owned()],
        },
        Entry::Tool {
            at: at(3),
            name: "read".to_owned(),
            subject: "src/lex.rs".to_owned(),
            took_us: Some(842),
            outcome: Outcome::Answered,
        },
        Entry::Tool {
            at: at(6),
            name: "exec".to_owned(),
            subject: "cargo test".to_owned(),
            took_us: Some(3_000_000),
            outcome: Outcome::Failed,
        },
        Entry::Reasoning { characters: 11 },
        Entry::Reply {
            said: "The lexer drops the last token.".to_owned(),
        },
        Entry::Ended {
            at: at(12),
            ending: Ending::Done,
            took_s: Some(12),
        },
    ];
    assert_eq!(entries, wanted);
    assert_eq!((seen.run, seen.calls.len()), (None, 0));
}

#[test]
fn a_call_under_way_is_live_until_it_answers() {
    let mut seen = Room::default();
    let started = [
        record(0, EventKind::RunStarted, serde_json::json!({ "task": "t" })),
        record(
            1,
            EventKind::ToolCalled,
            serde_json::json!({ "id": "a", "name": "exec", "args": {}, "subject": "cargo build" }),
        ),
    ];
    assert_eq!(read(&mut seen, &started), Vec::new());
    assert_eq!(seen.calls.len(), 1);
    assert_eq!(seen.since(), Some(TimeMs::new(3_600_000)));
}

#[test]
fn a_request_waits_live_and_reaches_the_transcript_when_answered() {
    let mut seen = Room::default();
    let asked = record(
        0,
        EventKind::ApprovalRequested,
        serde_json::json!({ "id": "q1", "actor": "lab/room1", "action_desc": "exec  rm -rf target",
            "artifact": "file:lab/room1@0123456789abcdef0123456789abcdef01234567",
            "cluster_key": { "class": "question", "detail": "ask the person" },
            "created": 0, "tainted": false }),
    );
    let answered = record(
        5,
        EventKind::ApprovalResolved,
        serde_json::json!({ "id": "q1", "verdict": "deny",
            "cluster": { "class": "question", "detail": "ask the person" } }),
    );
    assert_eq!(read(&mut seen, std::slice::from_ref(&asked)), Vec::new());
    assert_eq!(seen.waiting.len(), 1, "{asked:?}");
    assert_eq!(
        read(&mut seen, &[answered]),
        vec![Entry::Resolved {
            at: at(5),
            verdict: Verdict::Denied,
            what: "exec  rm -rf target".to_owned(),
        }]
    );
    assert!(seen.waiting.is_empty());
}
