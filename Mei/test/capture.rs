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

mod support;

use serde_json::Value;
use support::{serve, tool_result_text};

fn call(code: &str) -> Value {
    serde_json::json!({"stop_reason": "tool_use", "content": [
        {"type": "tool_use", "name": "Python3", "id": "call", "input": {"code": code}}
    ]})
}

fn run_with_temp(base: &str, prompt: &str, temporary: &std::path::Path) -> std::process::Output {
    support::command(base, prompt)
        .env("TMPDIR", temporary)
        .env("TEMP", temporary)
        .env("TMP", temporary)
        .output()
        .unwrap()
}

fn remaining_files(directory: &std::path::Path) -> usize {
    std::fs::read_dir(directory).unwrap().count()
}

#[test]
fn inline_capture_files_are_reclaimed_after_process_exit() {
    let temporary = tempfile::tempdir().unwrap();
    let (base, gateway) = serve(vec![
        call("print('small')"),
        serde_json::json!({"stop_reason": "end_turn", "content": "done"}),
    ]);
    let output = run_with_temp(&base, "small", temporary.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    gateway.join().unwrap();
    assert_eq!(remaining_files(temporary.path()), 0);
}

#[test]
fn a_failed_run_reclaims_its_complete_capture() {
    let temporary = tempfile::tempdir().unwrap();
    let (base, gateway) = serve(vec![
        call("print('x' * 20000)"),
        serde_json::json!({"stop_reason": "max_tokens", "content": []}),
    ]);
    let output = run_with_temp(&base, "fail", temporary.path());
    assert!(!output.status.success());
    gateway.join().unwrap();
    assert_eq!(remaining_files(temporary.path()), 0);
}

#[test]
fn spilled_output_remains_readable_after_interpreter_exit() {
    let temporary = tempfile::tempdir().unwrap();
    let (base, gateway) = serve(vec![
        call("print('x' * 20000)"),
        serde_json::json!({"stop_reason": "end_turn", "content": "done"}),
    ]);
    let output = run_with_temp(&base, "large", temporary.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = gateway.join().unwrap();
    let notice = tool_result_text(&requests[1]["messages"][2]["content"][0]["content"]);
    let path = notice.lines().last().unwrap();
    assert!(
        notice.starts_with("Output exceeds"),
        "unexpected fixture: {notice}"
    );
    assert!(std::fs::read_to_string(path).unwrap().len() > 16384);
}
