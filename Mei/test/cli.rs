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

#[cfg(unix)]
use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::io::{Read, Write};
use std::process::Command;
#[cfg(unix)]
use std::process::Output;

#[cfg(unix)]
use serde_json::Value;

#[cfg(unix)]
fn read_request_body(reader: &mut BufReader<std::net::TcpStream>) -> Vec<u8> {
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            panic!("client closed before sending complete request headers");
        }
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            length = value.trim().parse::<usize>().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    body
}

#[cfg(unix)]
fn serve(answers: Vec<Value>) -> (String, std::thread::JoinHandle<Vec<Value>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for answer in answers {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let body = read_request_body(&mut reader);
            requests.push(serde_json::from_slice(&body).unwrap());
            let body = serde_json::to_vec(&answer).unwrap();
            let mut stream = reader.into_inner();
            write!(
                stream,
                "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
            stream.shutdown(std::net::Shutdown::Both).unwrap();
        }
        requests
    });
    (format!("http://{address}"), handle)
}

#[cfg(unix)]
fn run(base: &str, prompt: &str) -> Output {
    run_with_harness(base, prompt, None)
}

#[cfg(unix)]
fn run_with_harness(base: &str, prompt: &str, harness: Option<&str>) -> Output {
    run_configured(base, prompt, harness, None)
}

#[cfg(unix)]
fn run_with_tools(base: &str, prompt: &str, tools: &str) -> Output {
    run_configured(base, prompt, None, Some(tools))
}

#[cfg(unix)]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn run_configured(base: &str, prompt: &str, harness: Option<&str>, tools: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mei"));
    command
        .args(["-p", prompt])
        .env("HARNESS_MODEL", "test/model")
        .env("ANTHROPIC_BASE_URL", base)
        .env("ANTHROPIC_API_KEY", "magpie")
        .env_remove("HARNESS_EXTRA")
        .env_remove("MEI_TOOLS");
    if let Some(harness) = harness {
        command.env("HARNESS", harness);
    } else {
        command.env_remove("HARNESS");
    }
    if let Some(tools) = tools {
        command.env("MEI_TOOLS", tools);
    }
    command.output().unwrap()
}

#[cfg(unix)]
#[test]
fn direct_answer_has_only_the_final_text_on_stdout() {
    let (base, handle) = serve(vec![
        serde_json::json!({"stop_reason": "end_turn", "content": [{"type": "text", "text": "answer"}]}),
    ]);
    let output = run(&base, "hello");
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

#[cfg(unix)]
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
    let output = run(&base, "compute");
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
    assert_eq!(results[0]["content"], "40\n");
    assert_eq!(results[1]["tool_use_id"], "b");
    assert_eq!(results[1]["content"], "42\n");
}

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn missing_model_is_an_actionable_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .arg("-p")
        .arg("hello")
        .env_remove("HARNESS_MODEL")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E_MEI_CONFIG"));
}

#[cfg(unix)]
#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn non_unicode_configuration_is_not_treated_as_unset() {
    use std::os::unix::ffi::OsStringExt;

    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["-p", "hello"])
        .env("HARNESS_MODEL", "test/model")
        .env("HARNESS_EXTRA", std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("E_MEI_CONFIG"), "{error}");
    assert!(error.contains("not Unicode"), "{error}");
}

#[cfg(unix)]
#[test]
fn a_preset_harness_is_not_overwritten() {
    let first = serde_json::json!({"stop_reason": "tool_use", "content": [
        {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "import os\nprint(os.environ['HARNESS'])"}}
    ]});
    let second = serde_json::json!({"stop_reason": "end_turn", "content": "done"});
    let (base, handle) = serve(vec![first, second]);
    let output = run_with_harness(&base, "compute", Some("/custom/wrapper"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = handle.join().unwrap();
    assert_eq!(
        requests[1]["messages"][2]["content"][0]["content"],
        "/custom/wrapper\n"
    );
}

#[cfg(unix)]
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
    let output = run(&base, "hello");
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "provider says no\n"
    );
    handle.join().unwrap();
}

#[cfg(unix)]
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
    let output = run(&base, "parent prompt");
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
        requests[2]["messages"][2]["content"][0]["content"],
        "child answer"
    );
}

#[cfg(unix)]
#[test]
fn pause_turn_preserves_transparent_server_tools_without_local_results() {
    let call = serde_json::json!({"type": "server_tool_use", "id": "s", "name": "web_search", "input": {"query": "Mei"}});
    let result = serde_json::json!({"type": "web_search_tool_result", "tool_use_id": "s", "content": "server result"});
    let (base, handle) = serve(vec![
        serde_json::json!({"stop_reason": "pause_turn", "content": [call.clone()]}),
        serde_json::json!({"stop_reason": "pause_turn", "content": [result.clone()]}),
        serde_json::json!({"stop_reason": "end_turn", "content": "done"}),
    ]);
    let output = run_with_tools(
        &base,
        "search",
        r#"[{"type":"web_search_20250305","name":"web_search","max_uses":1}]"#,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"done");
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0]["tools"][0]["name"], "python");
    assert_eq!(requests[0]["tools"][1]["name"], "web_search");
    assert_eq!(requests[1]["messages"].as_array().unwrap().len(), 2);
    assert_eq!(requests[2]["messages"].as_array().unwrap().len(), 3);
    assert_eq!(requests[2]["messages"][1]["content"][0], call);
    assert_eq!(requests[2]["messages"][2]["content"][0], result);
    assert!(requests.iter().all(|request| {
        !request["messages"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .any(|message| message["role"] == "user")
    }));
}

#[cfg(unix)]
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
            .env_remove("MEI_TOOLS")
            .stderr(trace)
            .output()
            .unwrap()
    };

    let first = playground.path().join("first");
    std::fs::create_dir(&first).unwrap();
    let (base, first_gateway) = serve(vec![
        serde_json::json!({"stop_reason": "tool_use", "content": [
            {"type": "tool_use", "name": "python", "id": "assign", "input": {"code": "answer = 40"}}
        ]}),
        serde_json::json!({"stop_reason": "tool_use", "content": [
            {"type": "tool_use", "name": "python", "id": "calculate", "input": {"code": "answer += 2\nprint(answer)"}}
        ]}),
        serde_json::json!({"stop_reason": "end_turn", "content": "42"}),
    ]);
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
path = Path("greeting.py")
path.write_text(path.read_text(encoding="utf-8").replace('return "hello"', 'return "hello Mei"'), encoding="utf-8")
result = subprocess.run(["python3", "-m", "unittest", "discover", "-v"], capture_output=True, text=True)
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
    assert!(trace.contains("OK"), "{trace}");
    assert_eq!(
        std::fs::read_to_string(project.join("greeting.py")).unwrap(),
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
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("session.jsonl");
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
        .env_remove("MEI_TOOLS")
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
