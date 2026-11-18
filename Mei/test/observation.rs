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

use serde_json::{Value, json};
use support::{run, run_resume, serve_with, tool_result_text};

fn tool(code: &str) -> Value {
    json!({"stop_reason": "tool_use", "content": [
        {"type": "tool_use", "name": "python", "id": "call", "input": {"code": code}}
    ]})
}

fn answer(text: &str) -> Value {
    json!({"stop_reason": "end_turn", "content": text})
}

#[test]
fn lowercase_tool_carries_exact_runtime_versions() {
    let (base, gateway) = serve_with(1, |_, _| answer("done"));
    let output = run(&base, "inspect", None);
    assert!(output.status.success());
    let requests = gateway.join().unwrap();
    assert_eq!(requests[0]["tools"][0]["name"], "python");
    let description = requests[0]["tools"][0]["description"].as_str().unwrap();
    assert!(description.starts_with("<python version=\""));
    assert!(description.contains("\" uv_version=\""));
}

#[test]
fn small_print_is_observable_by_the_next_model_request() {
    let (base, gateway) = serve_with(2, |request, index| {
        if index == 0 {
            return tool("print('observable-7f3c9a')");
        }
        let observed = tool_result_text(&request["messages"][2]["content"][0]["content"]);
        answer(if observed == "observable-7f3c9a\n" {
            "observable-7f3c9a"
        } else {
            "not observed"
        })
    });
    let output = run(&base, "read the marker", None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = gateway.join().unwrap();
    let received = tool_result_text(&requests[1]["messages"][2]["content"][0]["content"]);
    assert_eq!(
        output.stdout, b"observable-7f3c9a",
        "tool result: {received:?}"
    );
    assert_eq!(requests.len(), 2);
}

#[test]
fn resume_preserves_legacy_history_and_media_blocks() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.jsonl");
    let media = json!([
        {"type": "text", "text": "inspect"},
        {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "aW1hZ2U="}},
        {"type": "audio", "source": {"type": "base64", "media_type": "audio/wav", "data": "YXVkaW8="}},
        {"type": "video", "source": {"type": "url", "url": "https://example.invalid/video"}}
    ]);
    let entries = vec![
        json!({"role": "user", "content": media}),
        json!({"role": "assistant", "content": [
            {"type": "tool_use", "name": "python", "id": "legacy", "input": {"code": "print('old')"}}
        ]}),
        json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "legacy", "content": "C:\\legacy\\output.txt"}
        ]}),
        json!({"role": "assistant", "content": "done"}),
    ];
    let mut logged = entries.clone();
    logged[1]["stop_reason"] = json!("tool_use");
    logged[3]["stop_reason"] = json!("end_turn");
    std::fs::write(
        &path,
        logged
            .iter()
            .map(|entry| format!("{entry}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let (base, gateway) = serve_with(1, |_, _| answer("continued"));
    let output = run_resume(&base, "next", &path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"continued");
    let requests = gateway.join().unwrap();
    let expected = json!([
        entries[0], entries[1], entries[2], entries[3],
        {"role": "user", "content": "next"}
    ]);
    assert_eq!(requests[0]["messages"], expected);
    assert_eq!(requests[0]["tools"][0]["name"], "python");
}

#[test]
fn large_output_is_withheld_then_selectively_read() {
    let (base, gateway) = serve_with(3, |request, index| {
        if index == 0 {
            return tool("print('x' * 20000)");
        }
        if index == 1 {
            let notice = tool_result_text(&request["messages"][2]["content"][0]["content"]);
            let path = notice
                .strip_prefix("Output exceeds 16384 bytes. Use Python3 to filter and print the needed result.\n")
                .unwrap_or("");
            let literal = serde_json::to_string(path).unwrap();
            return tool(&format!(
                "from pathlib import Path\nprint(Path({literal}).read_text().splitlines()[-1][-4:])"
            ));
        }
        let observed = tool_result_text(&request["messages"][4]["content"][0]["content"]);
        answer(if observed == "xxxx\n" {
            "xxxx"
        } else {
            "not observed"
        })
    });
    let output = run(&base, "inspect the large result", None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = gateway.join().unwrap();
    let received = tool_result_text(&requests[2]["messages"][4]["content"][0]["content"]);
    assert_eq!(output.stdout, b"xxxx", "filtered result: {received:?}");
    assert_eq!(requests.len(), 3);
    let notice = tool_result_text(&requests[1]["messages"][2]["content"][0]["content"]);
    let path = notice.lines().last().unwrap();
    assert_eq!(
        std::fs::read_to_string(path)
            .unwrap()
            .replace("\r\n", "\n")
            .len(),
        20001
    );
}
