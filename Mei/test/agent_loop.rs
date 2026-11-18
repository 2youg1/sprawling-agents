// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufReader, Write};
use std::process::Command;

use serde_json::Value;

use crate::support::{read_request_body, run, serve, serve_with, tool_result_text};

#[test]
fn direct_answer_has_only_the_final_text_on_stdout() {
    let (base, handle) = serve(vec![
        serde_json::json!({"stop_reason": "end_turn", "content": [{"type": "text", "text": "answer"}]}),
    ]);
    let output = run(&base, "hello", None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"answer");
    let trace: Vec<Value> = String::from_utf8(output.stderr)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        trace,
        vec![
            serde_json::json!({"role": "user", "content": "hello"}),
            serde_json::json!({"role": "assistant", "content": [{"type": "text", "text": "answer"}], "stop_reason": "end_turn"})
        ]
    );
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["messages"][0]["content"], "hello");
}

#[test]
fn a_real_tool_wave_keeps_order_state_and_opaque_blocks() {
    let thinking =
        serde_json::json!({"type": "thinking", "thinking": "why", "signature": "signed"});
    let first = serde_json::json!({"stop_reason": "tool_use", "content": [
        thinking.clone(),
        {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "x = 40\nprint(x)"}},
        {"type": "tool_use", "name": "python", "id": "b", "input": {"code": "x += 2\nprint(x)"}}
    ]});
    let second = serde_json::json!({"stop_reason": "end_turn", "content": "42"});
    let (base, handle) = serve(vec![first, second]);
    let output = run(&base, "compute", None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"42");
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1]["messages"][1]["content"][0], thinking);
    let results = requests[1]["messages"][2]["content"].as_array().unwrap();
    assert_eq!(results[0]["tool_use_id"], "a");
    assert_eq!(tool_result_text(&results[0]["content"]), "40\n");
    assert_eq!(results[1]["tool_use_id"], "b");
    assert_eq!(tool_result_text(&results[1]["content"]), "42\n");
}

#[test]
fn a_preset_harness_is_not_overwritten() {
    let first = serde_json::json!({"stop_reason": "tool_use", "content": [
        {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "import os\nprint(os.environ['HARNESS'])"}}
    ]});
    let second = serde_json::json!({"stop_reason": "end_turn", "content": "done"});
    let (base, handle) = serve(vec![first, second]);
    let output = run(&base, "compute", Some("/custom/wrapper"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = handle.join().unwrap();
    assert_eq!(
        tool_result_text(&requests[1]["messages"][2]["content"][0]["content"]),
        "/custom/wrapper\n"
    );
}

#[test]
fn a_non_200_body_reaches_stderr_unchanged() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        drop(read_request_body(&mut reader));
        let mut stream = reader.into_inner();
        stream
            .write_all(b"HTTP/1.0 400 Bad Request\r\n\r\nprovider says no")
            .unwrap();
        stream.shutdown(std::net::Shutdown::Both).unwrap();
    });
    let output = run(&base, "hello", None);
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "provider says no\n"
    );
    handle.join().unwrap();
}

#[test]
fn a_recursive_child_has_an_independent_request_and_stdout_answer() {
    let parent = serde_json::json!({"stop_reason": "tool_use", "content": [
        {"type": "tool_use", "name": "python", "id": "parent", "input": {"code": "import os, subprocess\nchild = subprocess.run([os.environ['HARNESS'], '-p', 'child prompt'], capture_output=True, text=True)\nprint(child.stdout, end='')"}}
    ]});
    let (base, handle) = serve(vec![
        parent,
        serde_json::json!({"stop_reason": "end_turn", "content": "child answer"}),
        serde_json::json!({"stop_reason": "end_turn", "content": "parent done"}),
    ]);
    let output = run(&base, "parent prompt", None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"parent done");
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[1]["messages"],
        serde_json::json!([{"role": "user", "content": "child prompt"}])
    );
    assert_eq!(requests[2]["messages"][0]["content"], "parent prompt");
    assert_eq!(
        tool_result_text(&requests[2]["messages"][2]["content"][0]["content"]),
        "child answer"
    );
}

#[test]
fn pause_turn_continues_without_a_user_message() {
    let thinking = serde_json::json!({"type": "thinking", "thinking": "wait", "signature": "s"});
    let (base, handle) = serve(vec![
        serde_json::json!({"stop_reason": "pause_turn", "content": [thinking.clone()]}),
        serde_json::json!({"stop_reason": "end_turn", "content": "done"}),
    ]);
    let output = run(&base, "search", None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"done");
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["tools"][0]["name"], "python");
    assert_eq!(requests[1]["messages"].as_array().unwrap().len(), 2);
    assert_eq!(requests[1]["messages"][1]["content"][0], thinking);
}

#[test]
#[expect(clippy::disallowed_methods, reason = "end-to-end fixture process")]
fn e2e_beginner_workflow_from_first_prompt_to_project_change() {
    let playground = tempfile::tempdir().unwrap();
    let run = |directory: &std::path::Path, base: &str, prompt: &str| {
        let trace = std::fs::File::create(directory.join("run.jsonl")).unwrap();
        Command::new(env!("CARGO_BIN_EXE_mei"))
            .args(["-p", prompt])
            .current_dir(directory)
            .env("HARNESS_MODEL", "test/model")
            .env("ANTHROPIC_BASE_URL", base)
            .env("ANTHROPIC_API_KEY", "magpie")
            .env_remove("HARNESS")
            .env_remove("HARNESS_EXTRA")
            .stderr(trace)
            .output()
            .unwrap()
    };

    let first = playground.path().join("first");
    std::fs::create_dir(&first).unwrap();
    let (base, first_gateway) = serve_with(3, |request, index| match index {
        0 => serde_json::json!({"stop_reason": "tool_use", "content": [
            {"type": "tool_use", "name": "python", "id": "assign", "input": {"code": "answer = 40"}}
        ]}),
        1 => serde_json::json!({"stop_reason": "tool_use", "content": [
            {"type": "tool_use", "name": "python", "id": "calculate", "input": {"code": "answer += 2\nprint(answer)"}}
        ]}),
        2 => {
            let observed = tool_result_text(&request["messages"][4]["content"][0]["content"]);
            serde_json::json!({"stop_reason": "end_turn", "content": if observed == "42\n" { "42" } else { "not observed" }})
        }
        _ => panic!("unexpected gateway request"),
    });
    let output = run(&first, &base, "Use Python to calculate 40 + 2");
    let trace = std::fs::read_to_string(first.join("run.jsonl")).unwrap();
    assert!(output.status.success(), "{trace}");
    assert_eq!(output.stdout, b"42");
    assert_eq!(trace.lines().count(), 6);
    let requests = first_gateway.join().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0]["model"], "test/model");
    assert_eq!(requests[2]["messages"].as_array().unwrap().len(), 5);

    let project = playground.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(
        project.join("greeting.py"),
        "def greeting():\n    return \"hello\"\n",
    )
    .unwrap();
    std::fs::write(
        project.join("test_greeting.py"),
        "import unittest\nfrom greeting import greeting\n\nclass GreetingTest(unittest.TestCase):\n    def test_greeting(self):\n        self.assertEqual(greeting(), \"hello Mei\")\n\nif __name__ == \"__main__\":\n    unittest.main()\n",
    )
    .unwrap();
    let (base, project_gateway) = serve(vec![
        serde_json::json!({"stop_reason": "tool_use", "content": [
            {"type": "tool_use", "name": "python", "id": "repair", "input": {"code": r#"from pathlib import Path
import subprocess
import sys
path = Path("greeting.py")
path.write_text(path.read_text(encoding="utf-8").replace('return "hello"', 'return "hello Mei"'), encoding="utf-8")
result = subprocess.run([sys.executable, "-m", "unittest", "discover", "-v"], capture_output=True, text=True)
print(result.stdout, end="")
print(result.stderr, end="")
assert result.returncode == 0
"#}}
        ]}),
        serde_json::json!({"stop_reason": "end_turn", "content": "Updated the greeting and ran the tests."}),
    ]);
    let output = run(
        &project,
        &base,
        "Make greeting return hello Mei, then run the project tests",
    );
    let trace = std::fs::read_to_string(project.join("run.jsonl")).unwrap();
    assert!(output.status.success(), "{trace}");
    assert_eq!(output.stdout, b"Updated the greeting and ran the tests.");
    let repair = trace
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|row| {
            row.get("content")
                .and_then(Value::as_array)
                .and_then(|blocks| {
                    blocks.iter().find_map(|block| {
                        if block.get("type")?.as_str()? != "tool_result" {
                            return None;
                        }
                        Some(tool_result_text(block.get("content")?))
                    })
                })
        })
        .expect("repair tool_result in the JSONL trace");
    assert!(repair.contains("OK"), "{repair}");
    assert_eq!(
        std::fs::read_to_string(project.join("greeting.py"))
            .unwrap()
            .replace("\r\n", "\n"),
        "def greeting():\n    return \"hello Mei\"\n"
    );
    let requests = project_gateway.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0]["messages"],
        serde_json::json!([{"role": "user", "content": "Make greeting return hello Mei, then run the project tests"}])
    );
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "resume fixture process")]
fn cli_resume_imports_only_complete_message_history() {
    use std::os::unix::ffi::OsStrExt as _;

    let dir = tempfile::tempdir().unwrap();
    let session = dir
        .path()
        .join(std::ffi::OsStr::from_bytes(b"session-\xff.jsonl"));
    std::fs::write(
        &session,
        concat!(
            "{\"role\":\"user\",\"content\":\"first\"}\n",
            "{\"role\":\"assistant\",\"content\":\"answer\",\"stop_reason\":\"end_turn\"}\n"
        ),
    )
    .unwrap();
    let (base, gateway) = serve(vec![
        serde_json::json!({"stop_reason": "end_turn", "content": "continued"}),
    ]);
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["--resume"])
        .arg(&session)
        .args(["-p", "second"])
        .env("HARNESS_MODEL", "test/model")
        .env("ANTHROPIC_BASE_URL", &base)
        .env("ANTHROPIC_API_KEY", "magpie")
        .env_remove("HARNESS")
        .env_remove("HARNESS_EXTRA")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"continued");
    let requests = gateway.join().unwrap();
    assert_eq!(
        requests[0]["messages"],
        serde_json::json!([
            {"role": "user", "content": "first"},
            {"role": "assistant", "content": "answer"},
            {"role": "user", "content": "second"}
        ])
    );
    let trace: Vec<Value> = String::from_utf8(output.stderr)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        trace,
        vec![
            serde_json::json!({"role": "user", "content": "second"}),
            serde_json::json!({"role": "assistant", "content": "continued", "stop_reason": "end_turn"})
        ]
    );
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "invalid resume fixture process")]
fn cli_resume_rejects_an_incomplete_log_before_starting_python() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("incomplete.jsonl");
    std::fs::write(&session, "{\"role\":\"user\",\"content\":\"first\"}\n").unwrap();
    let marker = dir.path().join("python-started");
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let python = bin.join("python3");
    std::fs::write(
        &python,
        format!("#!/bin/sh\nprintf started > \"{}\"\n", marker.display()),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&python).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt as _;
    permissions.set_mode(0o755);
    std::fs::set_permissions(&python, permissions).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["--resume"])
        .arg(&session)
        .args(["-p", "second"])
        .current_dir(dir.path())
        .env("HARNESS_MODEL", "test/model")
        .env("ANTHROPIC_BASE_URL", "http://127.0.0.1:1")
        .env("ANTHROPIC_API_KEY", "magpie")
        .env("PATH", &bin)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_SESSION"), "{error}");
    assert!(!marker.exists(), "an invalid log reached Python");
}
