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

//! What a turn and a call carry beside their pairing, each read from
//! the one record that states it (`crates/wire/Spec.lean` §8-53 onward).

use super::turns;
use wire::{AxCode, AxError, B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq};
use wire::{TimeMs, Timing};

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

/// The same line as a build before per-line moments wrote it.
fn at_version_one(line: &EventRecord) -> EventRecord {
    let written = String::from_utf8(line.canonical_line().unwrap()).unwrap();
    let older = written.replacen("\"v\":2,", "\"v\":1,", 1);
    EventRecord::parse_line(older.as_bytes()).unwrap()
}

fn asked(seq: u64) -> EventRecord {
    record(seq, EventKind::ModelCalled, serde_json::json!({}))
}

fn returned(seq: u64, first_at: Option<u64>) -> EventRecord {
    let mut data = serde_json::json!({ "message": { "content": [] }, "calls": 0 });
    if let Some(at) = first_at {
        data["first_at"] = serde_json::json!(at);
    }
    record(seq, EventKind::ModelReturned, data)
}

fn called(seq: u64, id: &str) -> EventRecord {
    record(
        seq,
        EventKind::ToolCalled,
        serde_json::json!({ "id": id, "name": "exec", "args": { "cmd": "cargo test" } }),
    )
}

fn answered(seq: u64, id: &str) -> EventRecord {
    record(
        seq,
        EventKind::ToolResult,
        serde_json::json!({ "tool_use_id": id, "name": "exec", "result": { "exit_code": 0 } }),
    )
}

#[test]
fn a_turn_reports_when_its_first_content_arrived() {
    let folded = turns(&[asked(1), returned(4, Some(2))]);
    assert_eq!(folded[0].first_at, Some(TimeMs::new(2)));
    assert_eq!(
        turns(&[asked(1), returned(4, None)])[0].first_at,
        None,
        "a reply that recorded no first content has none to report"
    );
}

/// The reply's own moment is when it was whole; a line that measured no
/// moment of its own gives none, because its stamp is the turn's.
#[test]
fn a_turn_reports_when_its_reply_returned() {
    let folded = turns(&[asked(1), returned(4, Some(2))]);
    assert_eq!(folded[0].returned, Some(TimeMs::new(4)));
    let older = [
        at_version_one(&asked(1)),
        at_version_one(&returned(4, Some(2))),
    ];
    assert_eq!(
        turns(&older)[0].returned,
        None,
        "a version-one reply carries its turn's stamp, not when it returned"
    );
}

#[test]
fn a_ledger_written_before_per_line_moments_reads_as_unmeasured() {
    let current = [asked(1), called(2, "a"), answered(3, "a")];
    let folded = turns(&current);
    assert_eq!(
        (folded[0].timing, folded[0].calls[0].timing),
        (Timing::Measured, Timing::Measured)
    );

    let older: Vec<EventRecord> = current.iter().map(at_version_one).collect();
    let folded = turns(&older);
    assert_eq!(
        (folded[0].timing, folded[0].calls[0].timing),
        (Timing::Unmeasured, Timing::Unmeasured),
        "a version-one line carries its turn's stamp, not its own moment"
    );
    assert_eq!(
        folded[0].calls[0].answered,
        Some(TimeMs::new(3)),
        "the time still travels"
    );
}

#[test]
fn an_answer_the_city_supplied_after_a_restart_is_unmeasured() {
    let unknown = AxError::failure(
        AxCode::ToolOutcomeUnknown,
        "recover tool outcome",
        "exec (a)",
    )
    .with_recovery("verify the external state before retrying");
    let supplied = record(
        9,
        EventKind::ToolResult,
        serde_json::json!({ "tool_use_id": "a", "name": "exec",
                            "error": serde_json::to_value(&unknown).unwrap() }),
    );
    let folded = turns(&[asked(1), called(2, "a"), supplied]);
    assert_eq!(folded[0].calls[0].timing, Timing::Unmeasured);
    assert_eq!(
        folded[0].timing,
        Timing::Measured,
        "the turn's own moment is still measured"
    );
}

#[test]
fn a_call_says_what_its_tool_was_registered_as() {
    let registered = record(
        2,
        EventKind::ToolCalled,
        serde_json::json!({ "id": "a", "name": "exec", "args": { "cmd": "cargo test" },
                            "effect": "egress", "render": "terminal" }),
    );
    let folded = turns(&[asked(1), registered, called(3, "b")]);
    let drawn: Vec<(Option<kernel::Effect>, Option<kernel::RenderIntent>)> = folded[0]
        .calls
        .iter()
        .map(|call| (call.effect.clone(), call.render.clone()))
        .collect();
    assert_eq!(
        drawn,
        vec![
            (
                Some(kernel::Effect::Egress),
                Some(kernel::RenderIntent::Terminal)
            ),
            (None, None),
        ],
        "a line that recorded no registration is drawn as nothing in particular"
    );
}

#[test]
fn a_cut_output_points_at_its_original() {
    let whole = kernel::Locator::cas(B3Hash::digest(b"every line cargo wrote"));
    let account = serde_json::json!({ "original": whole.to_string(), "len": 4000,
                                      "substitute_len": 200, "rest_path": "lab/.rest/a.txt" });
    let cut = record(
        3,
        EventKind::ToolResult,
        serde_json::json!({ "tool_use_id": "a", "name": "exec",
                            "result": { "content": "cargo check: clean", "sieve": [account] } }),
    );
    let folded = turns(&[asked(1), called(2, "a"), cut]);
    let call = &folded[0].calls[0];
    assert_eq!(
        (
            call.output
                .as_ref()
                .and_then(|output| output.pinned.clone()),
            call.arguments
                .as_ref()
                .and_then(|given| given.pinned.clone())
        ),
        (Some(whole), None),
        "the output names where the whole of it is; the arguments never left"
    );
    let kept = turns(&[asked(1), called(2, "a"), answered(3, "a")]);
    assert_eq!(
        kept[0].calls[0]
            .output
            .as_ref()
            .and_then(|output| output.pinned.clone()),
        None
    );
}

/// An `exec` result names the code its command ended with; a command a
/// signal stopped has none, and the row then draws none
/// (`crates/wire/Spec.lean` §8-76).
#[test]
fn an_exec_call_carries_the_code_its_command_ended_with_and_no_code_when_it_had_none() {
    let exec = |seq: u64, id: &str| {
        record(
            seq,
            EventKind::ToolCalled,
            serde_json::json!({ "id": id, "name": "exec", "args": { "arm": "shell" } }),
        )
    };
    let result = |seq: u64, id: &str, result: serde_json::Value| {
        record(
            seq,
            EventKind::ToolResult,
            serde_json::json!({ "tool_use_id": id, "name": "exec", "result": result }),
        )
    };
    let folded = turns(&[
        asked(1),
        exec(2, "a"),
        exec(3, "b"),
        result(4, "a", serde_json::json!({ "content": "", "exit_code": 2 })),
        result(
            5,
            "b",
            serde_json::json!({ "content": "", "outcome": "signalled" }),
        ),
    ]);
    let codes: Vec<Option<i64>> = folded[0].calls.iter().map(|call| call.exit_code).collect();
    assert_eq!(codes, vec![Some(2), None]);
}

/// What the provider read from its cache and what it wrote into it are
/// priced apart, so a turn answers both (`crates/wire/Spec.lean` §8-76).
#[test]
fn a_turn_carries_cache_reads_and_cache_writes_apart() {
    let returned = record(
        2,
        EventKind::ModelReturned,
        serde_json::json!({ "message": { "content": [] }, "calls": 0, "usage": {
            "input_tokens": 1200, "output_tokens": 340,
            "cache_read_tokens": 800, "cache_write_tokens": 150 } }),
    );
    let used = turns(&[asked(1), returned])[0]
        .used
        .expect("usage is on the wire");
    assert_eq!(
        (used.cached, used.cache_write),
        (wire::Tokens::new(800), Some(wire::Tokens::new(150)))
    );
}
