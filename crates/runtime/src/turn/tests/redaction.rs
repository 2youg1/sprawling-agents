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
use kernel::ToolOutcome;
use serde_json::Value;

/// A shape-table key: the prefix the scanner knows, then ninety mixed
/// base62 bytes.
fn api_key() -> String {
    format!("sk-ant-{}", "a1B2c3D4e5".repeat(9))
}

fn read_env_call(key: &str) -> ToolCall {
    let mut args = serde_json::Map::new();
    args.insert("path".to_owned(), Value::String(".env".to_owned()));
    args.insert(
        "replace".to_owned(),
        Value::String(format!("OPENAI_API_KEY={key}")),
    );
    ToolCall {
        id: "call-7".to_owned(),
        name: kernel::ToolName::parse("edit").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

/// The ledger is append-only and exportable, so a key that reaches it
/// cannot be taken back. Tool arguments and tool results are the two
/// payloads most likely to quote one.
#[test]
fn a_key_in_tool_args_and_tool_result_never_reaches_the_ledger() {
    let key = api_key();
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel {
        calls: vec![read_env_call(&key)],
    };
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(7));
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
    let echoed = key.clone();
    let turn = advance(
        turn.execute(Interrupt::None, &mut ledger, &mut move |_call| {
            let mut result = serde_json::Map::new();
            result.insert(
                "stdout".to_owned(),
                Value::String(format!("wrote OPENAI_API_KEY={echoed}")),
            );
            Ok(ToolOutcome {
                result: Payload::new(result).unwrap(),
                attachments: Vec::new(),
            })
        })
        .unwrap(),
    );
    let PhaseOutcome::Advanced(report) = turn.record(Interrupt::None, &mut ledger).unwrap() else {
        panic!("the boundary was not interrupted");
    };

    // No ledger line holds the key, in any event of the turn.
    for line in &ledger.lines {
        let text = String::from_utf8(line.clone()).unwrap();
        assert!(!text.contains(&key), "a ledger line quoted the key: {text}");
    }

    // Everything else about the two events survives: the id, the tool
    // name, the argument that was not a secret, and the prose around
    // the replaced span.
    let called: Value = serde_json::from_slice(&ledger.lines[3]).unwrap();
    assert_eq!(called["kind"], "tool_called");
    assert_eq!(called["data"]["id"], "call-7");
    assert_eq!(called["data"]["name"], "edit");
    assert_eq!(called["data"]["args"]["path"], ".env");
    let replaced = called["data"]["args"]["replace"].as_str().unwrap();
    assert!(replaced.starts_with("OPENAI_API_KEY=secret:redacted/"));

    let result: Value = serde_json::from_slice(&ledger.lines[4]).unwrap();
    assert_eq!(result["kind"], "tool_result");
    assert_eq!(result["data"]["tool_use_id"], "call-7");
    assert_eq!(result["data"]["name"], "edit");
    let stdout = result["data"]["result"]["stdout"].as_str().unwrap();
    assert!(stdout.starts_with("wrote OPENAI_API_KEY=secret:redacted/"));

    // The run loop learns how many spans were kept out, never which.
    assert!(report.redacted() >= 2);

    // The chain still verifies: redaction happens before the hash.
    crate::replay::verify_lines(ledger.lines.clone()).unwrap();
}

/// The window keeps what the ledger drops: the model is answered with
/// the tool's real output, so the conversation still makes sense.
#[test]
fn the_wave_result_block_keeps_what_the_ledger_drops() {
    let key = api_key();
    let mut ledger = TestLedger::new();
    let mut model = OneShotModel {
        calls: vec![read_env_call(&key)],
    };
    let turn = Turn::begin(run_id(), "resident@sim.1".into(), TimeMs::new(8));
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
    let echoed = key.clone();
    let turn = advance(
        turn.execute(Interrupt::None, &mut ledger, &mut move |_call| {
            let mut result = serde_json::Map::new();
            result.insert("stdout".to_owned(), Value::String(echoed.clone()));
            Ok(ToolOutcome {
                result: Payload::new(result).unwrap(),
                attachments: Vec::new(),
            })
        })
        .unwrap(),
    );
    let PhaseOutcome::Advanced(report) = turn.record(Interrupt::None, &mut ledger).unwrap() else {
        panic!("the boundary was not interrupted");
    };
    match &report.wave_results()[0] {
        ContentBlock::ToolResult { content, .. } => assert!(content.contains(&key)),
        other => panic!("expected a tool_result block, got {other:?}"),
    }
}
