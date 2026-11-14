// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The only feedback loop (`Mei/spec/Agent.lean`). JSON is the sole
//! conversation state; closures and `Write` are the only outside faces.

use std::collections::BTreeSet;
use std::io::Write;

use serde_json::{Value, json};

use crate::error::{Error, Result, protocol};

/// Renders the value found at a failed field, naming absence plainly.
fn found(value: Option<&Value>) -> String {
    value.map_or_else(|| "<missing>".to_owned(), |value| value.to_string())
}

/// The path of the assistant message just appended, so a diagnostic can
/// name the offending content block.
fn message_path(request: &Value) -> String {
    request
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| messages.len().checked_sub(1))
        .map_or_else(
            || "messages".to_owned(),
            |index| format!("messages[{index}]"),
        )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Call {
    id: String,
    code: String,
}

/// Runs until the model ends a turn, or a transport, protocol, trace or
/// interpreter failure makes continuation invalid.
///
/// # Errors
/// Any adapter error, malformed control field, invalid client tool wave,
/// unexpected stop reason, or trace write failure stops the run. A failed
/// wave can already have external effects only after every call validated.
pub fn run(
    request: &mut Value,
    mut post: impl FnMut(&Value) -> Result<Value>,
    mut exec: impl FnMut(&str) -> Result<String>,
    trace: &mut dyn Write,
) -> Result<String> {
    let mut first_response = true;
    loop {
        let response = post(request)?;
        let stop = response
            .get("stop_reason")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                protocol(
                    "read response.stop_reason",
                    "stop_reason is missing or is not a string",
                    "return a complete Anthropic Messages response with a string stop_reason",
                )
                .with_path("response.stop_reason")
                .with_value(found(response.get("stop_reason")))
            })?
            .to_owned();
        let content = response.get("content").cloned().ok_or_else(|| {
            protocol(
                "read response.content",
                "content is missing",
                "return a complete Anthropic Messages response with content",
            )
            .with_path("response.content")
            .with_value(found(response.get("content")))
        })?;
        if !content.is_string() && !content.is_array() {
            return Err(protocol(
                "read response.content",
                "content is neither a string nor an array",
                "return text as a string or an array of content blocks",
            )
            .with_path("response.content")
            .with_value(found(Some(&content)))
            .into());
        }
        if first_response {
            trace_prompt(request, trace)?;
            first_response = false;
        }
        append(
            request,
            json!({"role": "assistant", "content": content.clone()}),
            Some(stop.as_str()),
            trace,
        )?;
        let base = message_path(request);
        match stop.as_str() {
            "pause_turn" => continue,
            "end_turn" => return text(&content, &base),
            "tool_use" => {
                let calls = calls(&content, &base)?;
                if calls.is_empty() {
                    return Err(protocol(
                        "validate a Python tool wave",
                        "stop_reason is tool_use but no client tool call is present",
                        "return at least one valid python tool_use block, or use another stop_reason",
                    )
                    .with_path(format!("{base}.content"))
                    .with_value(found(Some(&content)))
                    .into());
                }
                let mut results = Vec::with_capacity(calls.len());
                for call in calls {
                    let output = exec(&call.code)?;
                    results.push(json!({
                        "type": "tool_result",
                        "tool_use_id": call.id,
                        "content": output,
                    }));
                }
                append(
                    request,
                    json!({"role": "user", "content": results}),
                    None,
                    trace,
                )?;
            }
            "max_tokens" => {
                return Err(protocol(
                    "continue after max_tokens",
                    "the model exhausted max_tokens",
                    "raise max_tokens in HARNESS_EXTRA or finish the task in a smaller piece; no tool call was executed",
                )
                .with_path("response.stop_reason")
                .with_value("max_tokens")
                .into());
            }
            reason => {
                return Err(protocol(
                    "continue after a stop reason",
                    format!("unsupported stop_reason `{reason}`"),
                    "use a provider/model that returns end_turn, tool_use or pause_turn; this run was not continued",
                )
                .with_path("response.stop_reason")
                .with_value(reason)
                .into());
            }
        }
    }
}

fn append(
    request: &mut Value,
    message: Value,
    stop: Option<&str>,
    trace: &mut dyn Write,
) -> Result<()> {
    let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) else {
        return Err(protocol(
            "append a message",
            "request.messages is missing or is not an array",
            "construct one Messages request with messages as an array before calling agent::run",
        )
        .with_path("request.messages")
        .with_value(found(request.get("messages")))
        .into());
    };
    messages.push(message.clone());
    let logged = stop.map_or(message.clone(), |stop| {
        let mut logged = message;
        if let Some(object) = logged.as_object_mut() {
            object.insert("stop_reason".to_owned(), Value::from(stop));
        }
        logged
    });
    write_message(trace, &logged, "trace an appended message")
}

fn trace_prompt(request: &Value, trace: &mut dyn Write) -> Result<()> {
    let Some(prompt) = request
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| messages.last())
        .filter(|message| message["role"] == "user")
    else {
        return Ok(());
    };
    write_message(trace, prompt, "trace the initial user prompt")
}

fn write_message(trace: &mut dyn Write, message: &Value, action: &str) -> Result<()> {
    serde_json::to_writer(&mut *trace, message).map_err(|err| {
        protocol(action, err.to_string(), "keep stderr writable").with_path("trace")
    })?;
    trace
        .write_all(b"\n")
        .and_then(|()| trace.flush())
        .map_err(|err| {
            Error::from(
                protocol(action, err.to_string(), "keep stderr writable").with_path("trace"),
            )
        })
}

fn text(content: &Value, base: &str) -> Result<String> {
    if let Some(text) = content.as_str() {
        return Ok(text.to_owned());
    }
    let mut answer = String::new();
    for (index, block) in content.as_array().into_iter().flatten().enumerate() {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                let part = block.get("text").and_then(Value::as_str).ok_or_else(|| {
                    protocol(
                        "read a final text block",
                        "a text block has no string text",
                        "return every text block with a string text field",
                    )
                    .with_path(format!("{base}.content[{index}].text"))
                    .with_value(found(block.get("text")))
                })?;
                answer.push_str(part);
            }
            Some(_) => {}
            None => {
                return Err(protocol(
                    "read a content block type",
                    "a content block has no string type",
                    "return every content block with a string type",
                )
                .with_path(format!("{base}.content[{index}].type"))
                .with_value(found(block.get("type")))
                .into());
            }
        }
    }
    Ok(answer)
}

fn calls(content: &Value, base: &str) -> Result<Vec<Call>> {
    let mut calls = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, block) in content.as_array().into_iter().flatten().enumerate() {
        let kind = block.get("type").and_then(Value::as_str).ok_or_else(|| {
            protocol(
                "read a content block type",
                "a content block has no string type",
                "return every content block with a string type",
            )
            .with_path(format!("{base}.content[{index}].type"))
            .with_value(found(block.get("type")))
        })?;
        if kind != "tool_use" {
            continue;
        }
        let valid = block.get("name").and_then(Value::as_str) == Some("python");
        let id = block
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty());
        let code = block.pointer("/input/code").and_then(Value::as_str);
        let (Some(id), Some(code)) = (id, code) else {
            let (field, value) = if id.is_none() {
                ("id", found(block.get("id")))
            } else {
                ("input.code", found(block.pointer("/input/code")))
            };
            return Err(protocol(
                "validate a Python tool call",
                "tool_use id or input.code is missing or has the wrong type",
                "return a non-empty string id and a string input.code for every python call",
            )
            .with_path(format!("{base}.content[{index}].{field}"))
            .with_value(value)
            .into());
        };
        if !valid {
            return Err(protocol(
                "validate a client tool call",
                format!(
                    "unsupported client tool `{}`",
                    block
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>")
                ),
                "the only local client tool is python; server_tool_use blocks need no local result",
            )
            .with_path(format!("{base}.content[{index}].name"))
            .with_value(found(block.get("name")))
            .into());
        }
        if !ids.insert(id.to_owned()) {
            return Err(protocol(
                "validate a Python tool wave",
                format!("duplicate tool_use id `{id}`"),
                "give every tool_use in this wave a unique id; no call in the wave was executed",
            )
            .with_path(format!("{base}.content[{index}].id"))
            .with_value(id)
            .into());
        }
        calls.push(Call {
            id: id.to_owned(),
            code: code.to_owned(),
        });
    }
    Ok(calls)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    fn request() -> Value {
        json!({"model": "m", "messages": [{"role": "user", "content": "go"}]})
    }

    fn response(stop: &str, content: Value) -> Value {
        json!({"stop_reason": stop, "content": content})
    }

    fn drive(answers: Vec<Value>) -> (Result<String>, Value, String, Vec<String>) {
        let mut answers = VecDeque::from(answers);
        let mut request = request();
        let mut trace = Vec::new();
        let mut executed = Vec::new();
        let result = run(
            &mut request,
            |_| Ok(answers.pop_front().unwrap()),
            |code| {
                executed.push(code.to_owned());
                Ok(format!("out:{code}"))
            },
            &mut trace,
        );
        (result, request, String::from_utf8(trace).unwrap(), executed)
    }

    #[test]
    fn a_direct_answer_preserves_stdout_and_history() {
        let (answer, request, trace, executed) = drive(vec![response("end_turn", json!("answer"))]);
        assert_eq!(answer.unwrap(), "answer");
        assert!(executed.is_empty());
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
        assert_eq!(request["messages"][1]["content"], json!("answer"));
        let trace: Vec<Value> = trace
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            trace,
            vec![
                json!({"role": "user", "content": "go"}),
                json!({"role": "assistant", "content": "answer", "stop_reason": "end_turn"})
            ]
        );
    }

    #[test]
    fn a_wave_runs_in_order_and_preserves_thinking() {
        let thinking = json!({"type": "thinking", "thinking": "why", "signature": "signed"});
        let first = response(
            "tool_use",
            json!([
                thinking.clone(),
                {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "x = 40"}},
                {"type": "tool_use", "name": "python", "id": "b", "input": {"code": "x += 2; print(x)"}}
            ]),
        );
        let (answer, request, trace, executed) =
            drive(vec![first, response("end_turn", json!("42"))]);
        assert_eq!(answer.unwrap(), "42");
        assert_eq!(executed, ["x = 40", "x += 2; print(x)"]);
        assert_eq!(request["messages"][1]["content"][0], thinking);
        let results = request["messages"][2]["content"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["tool_use_id"], "a");
        assert_eq!(results[0]["content"], "out:x = 40");
        assert_eq!(results[1]["tool_use_id"], "b");
        assert_eq!(results[1]["content"], "out:x += 2; print(x)");
        assert_eq!(trace.lines().count(), 4);
    }

    #[test]
    fn pause_reposts_the_same_assistant_without_a_user_message() {
        let (answer, request, _, executed) = drive(vec![
            response(
                "pause_turn",
                json!([{"type": "thinking", "thinking": "wait", "signature": "s"}]),
            ),
            response("end_turn", json!("done")),
        ]);
        assert_eq!(answer.unwrap(), "done");
        assert!(executed.is_empty());
        assert_eq!(request["messages"].as_array().unwrap().len(), 3);
        assert_eq!(request["messages"][1]["role"], "assistant");
        assert_eq!(request["messages"][2]["role"], "assistant");
    }

    #[test]
    fn an_invalid_member_prevents_the_whole_wave() {
        let (answer, request, _, executed) = drive(vec![response(
            "tool_use",
            json!([
                {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "write()"}},
                {"type": "tool_use", "name": "shell", "id": "b", "input": {"code": "remove()"}}
            ]),
        )]);
        assert!(answer.is_err());
        assert!(executed.is_empty());
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_duplicate_id_prevents_the_whole_wave() {
        let (answer, _, _, executed) = drive(vec![response(
            "tool_use",
            json!([
                {"type": "tool_use", "name": "python", "id": "same", "input": {"code": "first()"}},
                {"type": "tool_use", "name": "python", "id": "same", "input": {"code": "second()"}}
            ]),
        )]);
        assert!(answer.is_err());
        assert!(executed.is_empty());
    }

    #[test]
    fn max_tokens_and_empty_waves_do_not_execute() {
        for answer in [
            response("max_tokens", json!([])),
            response("tool_use", json!([])),
        ] {
            let (result, _, _, executed) = drive(vec![answer]);
            assert!(result.is_err());
            assert!(executed.is_empty());
        }
    }

    #[test]
    fn an_execution_failure_does_not_invent_a_complete_result_message() {
        let mut request = request();
        let mut trace = Vec::new();
        let result = run(
            &mut request,
            |_| {
                Ok(response(
                    "tool_use",
                    json!([{"type": "tool_use", "name": "python", "id": "a", "input": {"code": "exit()"}}]),
                ))
            },
            |_| {
                Err(Error::from(protocol(
                    "execute Python",
                    "the interpreter died",
                    "start a new Mei process",
                )))
            },
            &mut trace,
        );
        assert!(result.is_err());
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
        assert_eq!(String::from_utf8(trace).unwrap().lines().count(), 2);
    }

    #[test]
    fn missing_control_fields_are_protocol_errors() {
        let (result, _, _, _) = drive(vec![json!({"content": "missing stop"})]);
        assert!(result.is_err());
        let (result, _, _, _) = drive(vec![response("end_turn", json!([{"type": "text"}]))]);
        assert!(result.is_err());
    }

    #[test]
    fn protocol_errors_name_the_offending_path_and_value() {
        let (result, _, _, _) = drive(vec![json!({"content": "missing stop"})]);
        let shown = result.unwrap_err().to_string();
        assert!(shown.contains("path = \"response.stop_reason\""), "{shown}");
        assert!(shown.contains("value = \"<missing>\""), "{shown}");
        let wave = response(
            "tool_use",
            json!([
                {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "first()"}},
                {"type": "tool_use", "name": "python", "input": {"code": "second()"}}
            ]),
        );
        let (result, _, _, executed) = drive(vec![wave]);
        assert!(executed.is_empty());
        let shown = result.unwrap_err().to_string();
        assert!(
            shown.contains("path = \"messages[1].content[1].id\""),
            "{shown}"
        );
    }
}
