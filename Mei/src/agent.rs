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

pub const TOOL_NAME: &str = "python";

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
    mut post: impl FnMut(&Value, &mut dyn Write) -> Result<Value>,
    mut exec: impl FnMut(&str) -> Result<String>,
    trace: &mut dyn Write,
) -> Result<String> {
    let mut first_response = true;
    loop {
        let response = post(request, trace)?;
        if let Some(usage) = response.get("usage") {
            write_message(
                trace,
                &json!({"type": "mei.obs", "kind": "usage", "usage": usage}),
                "trace a mei.obs usage line",
            )?;
        }
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
                        "validate a python tool wave",
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
        let valid = block.get("name").and_then(Value::as_str) == Some(TOOL_NAME);
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
                "validate a python tool call",
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
                "validate a python tool wave",
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
#[path = "agent_tests.rs"]
mod tests;
