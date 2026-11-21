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

const TINY_PNG: &str = r"import base64, struct, zlib

def chunk(kind, body):
    return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))

png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 1, 1, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(b'\x00\x00\x00\x00\x00')) + chunk(b'IEND', b'')
open('tiny.png', 'wb').write(png)
";

#[test]
fn show_image_returns_a_real_image_block_and_text_in_one_python_call() {
    let directory = tempfile::tempdir().unwrap();
    let code = format!("{TINY_PNG}show_image('tiny.png')\nprint(base64.b64encode(png).decode())");
    let (base, gateway) = serve_with(2, move |request, index| {
        if index == 0 {
            return tool(&code);
        }
        let content = &request["messages"][2]["content"][0]["content"];
        let encoded = content[0]["text"].as_str().unwrap_or_default().trim_end();
        let image_ok = content[1]["type"] == "image"
            && content[1]["source"]["media_type"] == "image/png"
            && content[1]["source"]["data"] == encoded;
        answer(if image_ok { "seen" } else { "missing" })
    });
    let output = support::command(&base, "inspect the picture")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"seen");
    assert_eq!(gateway.join().unwrap().len(), 2);
}

#[test]
fn a_picture_is_bound_to_the_call_that_attached_it() {
    let directory = tempfile::tempdir().unwrap();
    let first = format!(
        "{TINY_PNG}import threading\nready = threading.Event()\ngo = threading.Event()\noutcome = {{}}\ndef late():\n    ready.set()\n    go.wait()\n    try:\n        show_image('tiny.png')\n    except ValueError as e:\n        outcome['said'] = str(e)\nworker = threading.Thread(target=late)\nworker.start()\nready.wait()\nprint('spawned')"
    );
    let second = "go.set()\nworker.join()\nprint(outcome.get('said', 'no refusal'))";
    let (base, gateway) = serve_with(3, move |request, index| {
        if index == 0 {
            return tool(&first);
        }
        if index == 1 {
            return tool(second);
        }
        let late = tool_result_text(&request["messages"][4]["content"][0]["content"]);
        answer(
            if late.contains("attaches only during the call that started it") {
                "refused"
            } else {
                "attached"
            },
        )
    });
    let output = support::command(&base, "inspect the picture")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"refused");
    let requests = gateway.join().unwrap();
    assert_eq!(
        tool_result_text(&requests[1]["messages"][2]["content"][0]["content"]),
        "spawned\n"
    );
}

#[test]
fn one_call_attaches_at_most_two_pictures_and_names_the_limit() {
    let directory = tempfile::tempdir().unwrap();
    let code = format!(
        "{TINY_PNG}show_image('tiny.png')\nshow_image('tiny.png')\ntry:\n    show_image('tiny.png')\nexcept ValueError as e:\n    print(e)"
    );
    let (base, gateway) = serve_with(2, move |request, index| {
        if index == 0 {
            return tool(&code);
        }
        let content = &request["messages"][2]["content"][0]["content"];
        let blocks = content.as_array().expect("text and pictures");
        let limit = blocks[0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("at most 2 pictures");
        let shape =
            blocks.len() == 3 && blocks[1]["type"] == "image" && blocks[2]["type"] == "image";
        answer(if limit && shape { "seen" } else { "missing" })
    });
    let output = support::command(&base, "inspect the pictures")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"seen");
    assert_eq!(gateway.join().unwrap().len(), 2);
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
                .strip_prefix("Output exceeds 16384 bytes. Use python to filter and print the needed result.\n")
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
