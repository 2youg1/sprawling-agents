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

use super::super::*;
use super::helpers::*;
use crate::window::Opening;
use kernel::ToolOutcome;

#[test]
fn a_full_turn_appends_the_canonical_event_sequence() {
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel {
        calls: vec![probe_call()],
    };
    let mut window = Window::new();
    window.push_task_lines("probe the city", "one probe", Opening::FromJob);
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &window,
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
    let mut invoked = 0u32;
    let turn = advance(
        turn.execute(Interrupt::None, &mut ledger, &mut |_call| {
            invoked += 1;
            Ok(ToolOutcome {
                result: Payload::empty(),
            })
        })
        .unwrap(),
    );
    let PhaseOutcome::Advanced(report) = turn.record(Interrupt::None, &mut ledger).unwrap() else {
        panic!("the boundary was not interrupted");
    };
    assert_eq!(invoked, 1);
    assert_eq!(report.calls_made(), 1);
    assert_eq!(
        ledger.kinds(),
        [
            "prompt_assembled",
            "model_called",
            "model_returned",
            "tool_called",
            "tool_result"
        ]
    );
    assert_eq!(report.refs().len(), 5);
    assert_eq!(
        report.model_returned().kind(),
        kernel::EventKind::ModelReturned
    );
    // Window-folding material mirrors the ledger content.
    assert_eq!(report.assistant().len(), 1);
    assert_eq!(report.wave_results().len(), 1);
    match &report.wave_results()[0] {
        ContentBlock::ToolResult {
            tool_use_id,
            is_error,
            ..
        } => {
            assert_eq!(tool_use_id, "call-1");
            assert!(!is_error);
        }
        other => panic!("expected a tool_result block, got {other:?}"),
    }
    // model_called carries the duty name; tool events carry ids.
    let called: serde_json::Value = serde_json::from_slice(&ledger.lines[1]).unwrap();
    assert_eq!(called["data"]["model"], "script");
    let tool_called: serde_json::Value = serde_json::from_slice(&ledger.lines[3]).unwrap();
    assert_eq!(tool_called["data"]["id"], "call-1");
    let tool_result: serde_json::Value = serde_json::from_slice(&ledger.lines[4]).unwrap();
    assert_eq!(tool_result["data"]["tool_use_id"], "call-1");
}

#[test]
fn cancel_at_the_call_boundary_stops_before_any_model_bytes() {
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel { calls: vec![] };
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(1));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &Window::new(),
            &[],
            &shape(),
        )
        .unwrap(),
    );
    let outcome = turn
        .call(
            Interrupt::Cancel,
            &mut ledger,
            &mut model,
            &BuildingPolicy::default(),
            None,
        )
        .unwrap();
    match outcome {
        PhaseOutcome::Cancelled(cancelled) => {
            assert_eq!(
                cancelled.refs().last().unwrap().kind(),
                kernel::EventKind::CancelReceived
            );
        }
        PhaseOutcome::Advanced(_) => panic!("cancel must end the turn at the boundary"),
    }
    assert_eq!(ledger.kinds(), ["prompt_assembled", "cancel_received"]);
}

#[test]
fn steer_at_a_boundary_records_and_advances() {
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel { calls: vec![] };
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(4));
    let turn = advance(
        turn.assemble(
            Interrupt::Steer {
                source: "user".to_owned(),
                text: "prefer the short route".to_owned(),
            },
            &mut ledger,
            &prefix(),
            &Window::new(),
            &[],
            &shape(),
        )
        .unwrap(),
    );
    let _ = advance(
        turn.call(
            Interrupt::None,
            &mut ledger,
            &mut model,
            &BuildingPolicy::default(),
            None,
        )
        .unwrap(),
    );
    assert_eq!(
        ledger.kinds(),
        [
            "steer_received",
            "prompt_assembled",
            "model_called",
            "model_returned"
        ]
    );
    let steer: serde_json::Value = serde_json::from_slice(&ledger.lines[0]).unwrap();
    assert_eq!(steer["data"]["source"], "user");
    assert_eq!(steer["data"]["text"], "prefer the short route");
}
#[test]
fn a_tool_error_lands_in_tool_result_not_in_the_turn() {
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel {
        calls: vec![ToolCall {
            id: "call-9".to_owned(),
            name: kernel::ToolName::parse("broken").unwrap(),
            args: Payload::empty(),
        }],
    };
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(2));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &Window::new(),
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
        turn.execute(Interrupt::None, &mut ledger, &mut |call| {
            Err(AxError::failure(
                AxCode::ToolUnavailable,
                "invoke tool",
                call.name.to_string(),
            ))
        })
        .unwrap(),
    );
    let PhaseOutcome::Advanced(report) = turn.record(Interrupt::None, &mut ledger).unwrap() else {
        panic!("the boundary was not interrupted");
    };
    assert_eq!(report.calls_made(), 1);
    let last = ledger.lines.last().unwrap();
    let value: serde_json::Value = serde_json::from_slice(last).unwrap();
    assert_eq!(value["kind"], "tool_result");
    assert_eq!(value["data"]["error"]["code"], "E_TOOL_UNAVAILABLE");
    match &report.wave_results()[0] {
        ContentBlock::ToolResult { is_error, .. } => assert!(is_error),
        other => panic!("expected a tool_result block, got {other:?}"),
    }
}

#[test]
fn the_ledger_chain_stays_verifiable_after_a_turn() {
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel { calls: vec![] };
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(3));
    let turn = advance(
        turn.assemble(
            Interrupt::None,
            &mut ledger,
            &prefix(),
            &Window::new(),
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
        turn.execute(Interrupt::None, &mut ledger, &mut |_call| {
            panic!("empty wave must not invoke")
        })
        .unwrap(),
    );
    let _report = turn.record(Interrupt::None, &mut ledger).unwrap();
    crate::replay::verify_lines(ledger.lines.clone()).unwrap();
}
