// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use mei::launch::{SYSTEM, initial_request, parse_extra, python_tool};

fn environment() -> doctor::Environment {
    doctor::Environment::new(
        doctor::Program::new("3.12.13", std::path::PathBuf::from("python")),
        doctor::Program::new("0.12.3", std::path::PathBuf::from("uv")),
    )
}

fn request(task: Task, extra: Option<&str>) -> Value {
    initial_request(task, parse_extra(extra).unwrap(), &environment()).unwrap()
}

#[test]
fn the_request_has_one_state_and_the_prompt_owns_messages() {
    let request = request(
        Task {
            model: "provider/model",
            prompt: "new prompt",
            history: None,
            guide: None,
        },
        Some(r#"{"system":"custom","messages":[{"role":"user","content":"old"}],"max_tokens":42}"#),
    );
    assert_eq!(request["model"], "provider/model");
    assert_eq!(request["system"], "custom");
    assert_eq!(request["max_tokens"], 42);
    assert_eq!(request["messages"][0]["content"], "new prompt");
    assert_eq!(request["tools"].as_array().unwrap().len(), 1);
    assert_eq!(request["tools"][0]["name"], "python");
    assert_eq!(
        request["tools"][0]["description"],
        python::tool_description(&environment())
    );
    assert!(request["tools"][0].get("strict").is_none());
}

#[test]
fn resumed_messages_precede_the_new_prompt() {
    let request = request(
        Task {
            model: "provider/model",
            prompt: "new",
            history: Some(vec![
                json!({"role": "user", "content": "old"}),
                json!({"role": "assistant", "content": "answer"}),
            ]),
            guide: Some("must-not-be-injected"),
        },
        None,
    );
    assert_eq!(request["max_tokens"], 256_000);
    assert_eq!(
        request["messages"],
        json!([
            {"role": "user", "content": "old"},
            {"role": "assistant", "content": "answer"},
            {"role": "user", "content": "new"}
        ])
    );
}

#[test]
fn readmes_match_the_documented_request_fixture() {
    let tool = python_tool(&environment());
    let pretty = serde_json::to_string_pretty(&tool).unwrap();
    let tool_bytes = serde_json::to_vec(&tool).unwrap().len();
    let total = SYSTEM.len().checked_add(tool_bytes).unwrap();
    for readme in [
        include_str!("../README.md"),
        include_str!("../README.zh-CN.md"),
    ] {
        assert!(readme.contains(SYSTEM));
        assert!(readme.contains(&pretty));
        for bytes in [SYSTEM.len(), tool_bytes, total] {
            assert!(readme.contains(&format!("| {bytes} |")));
        }
    }
}

#[test]
fn forbidden_configuration_is_refused_before_any_side_effect() {
    assert!(parse_extra(Some("[]")).is_err());
    assert!(parse_extra(Some(r#"{"stream":true}"#)).is_err());
    assert!(parse_extra(Some(r#"{"tools":[]}"#)).is_err());
}
