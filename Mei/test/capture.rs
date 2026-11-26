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
use support::{serve, serve_with, tool_result_text};

fn call(code: &str) -> Value {
    serde_json::json!({"stop_reason": "tool_use", "content": [
        {"type": "tool_use", "name": "python", "id": "call", "input": {"code": code}}
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
fn inline_file_counts_do_not_grow_at_n_and_twice_n() {
    for count in [4, 8] {
        let temporary = tempfile::tempdir().unwrap();
        let mut responses = (0..count).map(|_| call("pass")).collect::<Vec<_>>();
        responses.push(serde_json::json!({"stop_reason": "end_turn", "content": "done"}));
        let (base, gateway) = serve(responses);
        let output = run_with_temp(&base, "many", temporary.path());
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        gateway.join().unwrap();
        assert_eq!(remaining_files(temporary.path()), 0, "count={count}");
    }
}

#[test]
fn an_existing_directory_is_never_reused_or_deleted() {
    let temporary = tempfile::tempdir().unwrap();
    let existing = temporary.path().join("existing");
    std::fs::create_dir(&existing).unwrap();
    let marker = existing.join("keep");
    std::fs::write(&marker, "keep").unwrap();
    let (base, _gateway) = serve(vec![]);
    let output = support::command(&base, "task")
        .env("MEI_CAPTURE_DIR", &existing)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E_MEI_CAPTURE"));
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "keep");
}

#[test]
fn a_calyx_created_empty_directory_is_adopted() {
    let directory = tempfile::tempdir().unwrap();
    let created = directory.path().join("created");
    std::fs::create_dir(&created).unwrap();
    let (base, gateway) = serve(vec![
        serde_json::json!({"stop_reason": "end_turn", "content": "done"}),
    ]);
    let output = support::command(&base, "task")
        .env("MEI_CAPTURE_DIR", &created)
        .env("MEI_CAPTURE_CREATED", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    gateway.join().unwrap();
    assert!(!created.exists());
}

#[test]
fn an_empty_capture_override_has_an_exact_repair() {
    let (base, _gateway) = serve(vec![]);
    let output = support::command(&base, "task")
        .env("MEI_CAPTURE_DIR", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let shown = String::from_utf8_lossy(&output.stderr);
    assert!(shown.contains("E_MEI_CAPTURE"), "{shown}");
    assert!(shown.contains("value = \\\"\\\""), "{shown}");
    let repair = if cfg!(windows) {
        "Remove-Item Env:MEI_CAPTURE_DIR"
    } else {
        "unset MEI_CAPTURE_DIR"
    };
    assert!(shown.contains(repair), "{shown}");
}

#[cfg(unix)]
#[test]
fn a_non_unicode_capture_root_round_trips_through_python() {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
    let directory = tempfile::tempdir().unwrap();
    let mut bytes = directory.path().as_os_str().as_bytes().to_vec();
    bytes.extend_from_slice(b"/capture-\xff");
    let root = std::path::PathBuf::from(std::ffi::OsString::from_vec(bytes));
    let (base, gateway) = serve_with(4, move |request, index| match index {
        0 => call("print('small')"),
        1 => call("print('x' * 20000)"),
        2 => {
            let notice = tool_result_text(&request["messages"][4]["content"][0]["content"]);
            let expression = notice.lines().last().unwrap();
            call(&format!("print(len(open({expression}, 'rb').read()))"))
        }
        3 => {
            let observed = tool_result_text(&request["messages"][6]["content"][0]["content"]);
            serde_json::json!({"stop_reason": "end_turn", "content": if observed == "20001\n" { "readable" } else { "missing" }})
        }
        _ => panic!("unexpected request"),
    });
    let output = support::command(&base, "non-unicode capture")
        .env("MEI_CAPTURE_DIR", &root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"readable");
    gateway.join().unwrap();
    assert!(root.exists());
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

#[test]
fn a_resumed_process_can_read_the_original_spilled_path() {
    let sandbox = tempfile::tempdir().unwrap();
    let first_temp = sandbox.path().join("first-temp");
    let second_temp = sandbox.path().join("second-temp");
    std::fs::create_dir(&first_temp).unwrap();
    std::fs::create_dir(&second_temp).unwrap();
    let session = sandbox.path().join("session.jsonl");
    let (base, first) = serve(vec![
        call("print('x' * 20000)"),
        serde_json::json!({"stop_reason": "end_turn", "content": "done"}),
    ]);
    let output = support::command(&base, "large")
        .env("TMPDIR", &first_temp)
        .env("TEMP", &first_temp)
        .env("TMP", &first_temp)
        .stderr(std::fs::File::create(&session).unwrap())
        .output()
        .unwrap();
    assert!(output.status.success());
    let first_requests = first.join().unwrap();
    let notice = tool_result_text(&first_requests[1]["messages"][2]["content"][0]["content"]);
    let path = notice.lines().last().unwrap().to_owned();
    let literal = serde_json::to_string(&path).unwrap();
    let read = format!(
        "from pathlib import Path\nprint(Path({literal}).read_text().splitlines()[-1][-4:])"
    );
    let (base, second) = serve_with(2, move |request, index| {
        if index == 0 {
            return call(&read);
        }
        let observed = tool_result_text(&request["messages"][6]["content"][0]["content"]);
        serde_json::json!({"stop_reason": "end_turn", "content": if observed == "xxxx\n" { "readable" } else { "missing" }})
    });
    let output = support::command(&base, "read old output")
        .arg("--resume")
        .arg(&session)
        .env("TMPDIR", &second_temp)
        .env("TEMP", &second_temp)
        .env("TMP", &second_temp)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"readable");
    second.join().unwrap();
    assert!(std::fs::read_to_string(path).unwrap().len() > 16384);
}

#[test]
fn a_text_turn_finishes_without_starting_the_cached_interpreter() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("unavailable.exe");
    std::fs::write(&executable, []).unwrap();
    let program = mei::doctor::Program::new("3.12.13", executable);
    mei::doctor::write_cache(
        directory.path(),
        &mei::doctor::Environment::new(program.clone(), program),
    )
    .unwrap();
    for (response, needs_python) in [
        (
            serde_json::json!({"stop_reason": "end_turn", "content": "done"}),
            false,
        ),
        (call("print(42)"), true),
    ] {
        let (base, gateway) = serve(vec![response]);
        let output = support::command(&base, "continue")
            .env("MEI_DOCTOR_DIR", directory.path())
            .output()
            .unwrap();
        assert_eq!(output.status.success(), !needs_python);
        if needs_python {
            assert!(String::from_utf8_lossy(&output.stderr).contains("E_MEI_PYTHON"));
        } else {
            assert_eq!(output.stdout, b"done");
        }
        assert_eq!(gateway.join().unwrap().len(), 1);
    }
}
