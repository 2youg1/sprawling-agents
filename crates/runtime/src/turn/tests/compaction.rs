// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The turn boundary's compaction: the snapshot is replaced once, at the
//! closing boundary of `record`, and never while the wave is landing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]

use super::super::*;
use super::helpers::*;
use crate::compaction::Exchange;
use kernel::ToolOutcome;
use kernel::event::record::ModelReturned;

/// A model that says as much as the test needs and then calls what it is
/// told to.
struct VerboseModel {
    text: String,
    calls: Vec<ToolCall>,
}

impl Model for VerboseModel {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: self.text.clone(),
            }])?,
            std::mem::take(&mut self.calls),
        ))
    }
}

/// Deterministic prose of at least `len` bytes, cut on a word end.
fn prose(len: usize) -> String {
    let unit = "lorem ipsum dolor sit amet ";
    let mut out = unit.repeat(len / unit.len() + 1);
    out.truncate(len);
    out
}

/// What one wave result's text looks like in the window and on the
/// ledger: one printed JSON payload, printed the way the wave prints it.
fn answer(note: usize) -> String {
    let payload = Payload::of(&serde_json::json!({ "note": prose(note) })).unwrap();
    serde_json::to_string(&payload).unwrap()
}

fn outcome(text: &str) -> ToolOutcome {
    ToolOutcome {
        result: Payload::of(&serde_json::json!({ "note": text })).unwrap(),
        attachments: Vec::new(),
    }
}

/// The exchange the turn actually produced, rebuilt from the same
/// values the test fed it, then compacted through the same door.
fn expected(reply: &str, answers: &[String]) -> Exchange {
    let mut exchange = Exchange::new();
    exchange.push_assistant(vec![ContentBlock::Text {
        text: reply.to_owned(),
    }]);
    for one in answers {
        exchange.push_result(ContentBlock::ToolResult {
            tool_use_id: "call-1".to_owned(),
            content: one.clone(),
            is_error: false,
            attachments: Vec::new(),
        });
    }
    exchange.compact().unwrap();
    exchange
}

fn assistant_text(blocks: &[ContentBlock]) -> String {
    match &blocks[0] {
        ContentBlock::Text { text } => text.clone(),
        other => panic!("expected a text block, got {other:?}"),
    }
}

#[test]
fn the_snapshot_is_replaced_at_the_closing_boundary_and_not_before() {
    let reply = prose(60_000);
    let mut ledger = TestLedger::new();
    let mut model = VerboseModel {
        text: reply.clone(),
        calls: vec![probe_call()],
    };
    let mut conversation = Conversation::new();
    conversation.push_task_lines(
        "say a lot",
        "one long reply",
        crate::conversation::Opening::FromJob,
    );
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &conversation,
            &[],
            &shape(),
        )
        .unwrap(),
    );
    let turn = advance(
        turn.call(
            Interrupt::None,
            &mut ledger,
            &mut model,
            &BuildingPolicy::default(),
            None,
        )
        .unwrap(),
    );
    let turn = advance(
        turn.execute_concurrent(
            Interrupt::None,
            &mut ledger,
            &mut |_: &ToolCall, _: TimeMs| Ok(outcome(&prose(20))),
            &mut |_| Interrupt::None,
        )
        .unwrap(),
    );
    let PhaseOutcome::Advanced(report) = turn.record(Interrupt::None, &mut ledger).unwrap() else {
        panic!("the boundary was not interrupted");
    };

    // Everything written before the closing boundary carries the full
    // bytes: `model_returned` and `tool_result` are the wave's own
    // records, and the snapshot is not replaced under them.
    for line in &ledger.lines {
        let written = String::from_utf8_lossy(line).into_owned();
        let value: serde_json::Value = serde_json::from_str(&written).unwrap();
        match value["kind"].as_str().unwrap() {
            "model_returned" => {
                let returned: ModelReturned =
                    serde_json::from_value(value["data"].clone()).unwrap();
                let carried = serde_json::to_string(&returned.message).unwrap();
                assert!(carried.contains(&reply), "the ledger kept the full reply");
            }
            "tool_result" => assert!(
                written.contains(&answer(20)),
                "the ledger kept the full result"
            ),
            _ => {}
        }
    }

    // The closing boundary is where the snapshot is replaced.
    let compacted = assistant_text(report.assistant());
    assert!(!compacted.is_empty());
    assert!(compacted.len() < reply.len(), "the boundary compacted");
    assert_eq!(
        compacted,
        assistant_text(expected(&reply, &[answer(20)]).assistant())
    );
}

#[test]
fn a_threshold_crossed_mid_wave_compacts_once_over_the_whole_exchange() {
    let reply = prose(25_000);
    let first = answer(20_000);
    let second = answer(20_000);
    let mut ledger = TestLedger::new();
    let mut model = VerboseModel {
        text: reply.clone(),
        calls: vec![
            ToolCall {
                id: "call-1".to_owned(),
                name: kernel::ToolName::parse("probe").unwrap(),
                args: Payload::empty(),
            },
            ToolCall {
                id: "call-2".to_owned(),
                name: kernel::ToolName::parse("probe").unwrap(),
                args: Payload::empty(),
            },
        ],
    };
    let answers = [first.clone(), second.clone()];
    let mut conversation = Conversation::new();
    conversation.push_task_lines(
        "cross mid-wave",
        "one over-budget wave",
        crate::conversation::Opening::FromJob,
    );
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &conversation,
            &[],
            &shape(),
        )
        .unwrap(),
    );
    let turn = advance(
        turn.call(
            Interrupt::None,
            &mut ledger,
            &mut model,
            &BuildingPolicy::default(),
            None,
        )
        .unwrap(),
    );
    let mut seen = 0usize;
    let turn = advance(
        turn.execute_concurrent(
            Interrupt::None,
            &mut ledger,
            &mut |_call: &ToolCall, _: TimeMs| {
                seen += 1;
                Ok(outcome(&prose(20_000)))
            },
            &mut |_| Interrupt::None,
        )
        .unwrap(),
    );
    assert_eq!(seen, 2);
    let PhaseOutcome::Advanced(report) = turn.record(Interrupt::None, &mut ledger).unwrap() else {
        panic!("the boundary was not interrupted");
    };

    // The first result already pushed the exchange past its budget — the
    // threshold was crossed mid-wave. What the report carries is the
    // compaction of the whole exchange, not of the half-landed one a
    // mid-wave compactor would have met and frozen in.
    let whole = assistant_text(expected(&reply, &answers).assistant());
    let half_landed = assistant_text(expected(&reply, &answers[..1]).assistant());
    assert_ne!(whole, half_landed, "the two groupings disagree");
    assert_eq!(assistant_text(report.assistant()), whole);
}
