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

use crate::worker::fixture::*;
use crate::worker::*;

/// The last tool answer the scripted model actually received. This
/// checks rendered window bytes, not merely the ledger's JSON payload.
fn latest_tool(body: &serde_json::Value) -> &str {
    body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|message| message["role"] == "tool")
        .and_then(|message| message["content"].as_str())
        .unwrap()
}

fn two_reads(path: &str) -> String {
    serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "read the same file concurrently",
                "tool_calls": (["tu_wave_a", "tu_wave_b"].map(|id| serde_json::json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": "read",
                        "arguments": serde_json::json!({"file_path":path}).to_string()
                    }
                }))),
            },
            "finish_reason": "tool_calls",
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
    })
    .to_string()
}

#[test]
fn a_real_agent_reuses_a_read_but_receives_the_full_text_after_its_edit() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    let original: String = (0..300)
        .map(|line| format!("line {line}: a substantial passage\n"))
        .collect();
    let path = dir.path().join("lab/nested/long.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &original).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "read",
                "read",
                "tu_read_1",
                serde_json::json!({"file_path":"lab/nested/long.txt"}),
            ),
            completion_with(
                "read again",
                "read",
                "tu_read_2",
                serde_json::json!({"file_path":"lab/nested/long.txt"}),
            ),
            completion_with(
                "edit",
                "edit",
                "tu_edit",
                serde_json::json!({
                    "file_path":"lab/nested/long.txt",
                    "old_string":"line 10: a substantial passage\n",
                    "new_string":"LINE 10: the edited passage\n"
                }),
            ),
            completion_with(
                "check the edit",
                "read",
                "tu_read_3",
                serde_json::json!({"file_path":"lab/nested/long.txt"}),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/nested").unwrap(),
            task: "read two files".to_owned(),
            goal: "reuse unchanged reads without hiding an edit".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    let bodies: Vec<serde_json::Value> = provider
        .bodies()
        .iter()
        .filter_map(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .filter(|body| body.get("messages").is_some())
        .collect();
    assert_eq!(bodies.len(), 5, "five scripted turns: {bodies:?}");
    for body in &bodies[1..] {
        assert_eq!(body["system"], bodies[0]["system"]);
        assert_eq!(body["tools"], bodies[0]["tools"]);
    }
    let first = latest_tool(&bodies[1]);
    assert!(
        first.starts_with("lab/nested/long.txt lines 1-300 of 300\n"),
        "{first}"
    );
    assert!(first.contains("line 10: a substantial passage"), "{first}");

    let repeated = latest_tool(&bodies[2]);
    assert_eq!(
        repeated,
        "lab/nested/long.txt lines 1-300 of 300: unchanged since your earlier read; not repeated\n"
    );

    let edited = latest_tool(&bodies[3]);
    assert!(
        edited.contains("-line 10: a substantial passage"),
        "{edited}"
    );
    assert!(edited.contains("+LINE 10: the edited passage"), "{edited}");
    let checked = latest_tool(&bodies[4]);
    assert!(
        checked.starts_with("lab/nested/long.txt lines 1-300 of 300\n"),
        "{checked}"
    );
    assert!(checked.contains("LINE 10: the edited passage"), "{checked}");
    assert!(
        !checked.contains("unchanged since your earlier read"),
        "{checked}"
    );
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        original.replacen(
            "line 10: a substantial passage\n",
            "LINE 10: the edited passage\n",
            1,
        )
    );
    let reads: Vec<serde_json::Value> =
        storage::read_raw_lines_at(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap()
            .iter()
            .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
            .filter(|line| line["kind"] == "tool_result" && line["data"]["name"] == "read")
            .collect();
    assert_eq!(reads.len(), 3, "{reads:?}");
    let stub = &reads[1]["data"];
    assert_eq!(stub["rendered"], "v4");
    assert_eq!(stub["result"]["unchanged"], true);
    assert_eq!(stub["result"]["offset"], 0);
    assert_eq!(stub["result"]["lines"], 300);
    assert_eq!(stub["result"]["total_lines"], 300);
    for absent in ["text", "bytes", "next_offset", "rule_context"] {
        assert_eq!(stub["result"].get(absent), None, "{absent}: {stub}");
    }
    assert_eq!(reads[2]["data"]["result"].get("unchanged"), None);
}

#[test]
fn duplicate_reads_in_one_wave_both_return_text_before_the_next_wave_can_reuse_them() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    let path = dir.path().join("lab/nested/long.txt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        (0..300)
            .map(|line| format!("line {line}: a substantial passage\n"))
            .collect::<String>(),
    )
    .unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            two_reads("lab/nested/long.txt"),
            completion_with(
                "read after the wave",
                "read",
                "tu_after_wave",
                serde_json::json!({"file_path":"lab/nested/long.txt"}),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/nested").unwrap(),
            task: "read two files".to_owned(),
            goal: "do not let speculative reads consume their own evidence".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    let bodies: Vec<serde_json::Value> = provider
        .bodies()
        .iter()
        .filter_map(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .filter(|body| body.get("messages").is_some())
        .collect();
    assert_eq!(bodies.len(), 3, "{bodies:?}");
    let wave: Vec<&str> = bodies[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "tool")
        .filter_map(|message| message["content"].as_str())
        .collect();
    assert_eq!(wave.len(), 2, "{wave:?}");
    for answer in wave {
        assert!(
            answer.starts_with("lab/nested/long.txt lines 1-300 of 300\n"),
            "{answer}"
        );
        assert!(!answer.contains("unchanged since"), "{answer}");
    }
    assert_eq!(
        latest_tool(&bodies[2]),
        "lab/nested/long.txt lines 1-300 of 300: unchanged since your earlier read; not repeated\n"
    );
}
