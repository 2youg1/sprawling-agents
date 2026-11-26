// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Map, Value, json};

use crate::error::{Result, protocol};
use crate::observation;
use crate::python::{Execution, Py};
use crate::request::{Request, Transcript};
use crate::steer::{self, Intake};
use crate::transport::Client;

pub const TOOL_NAME: &str = "python";

pub(crate) trait Ports {
    fn post(&mut self, request: &dyn Transcript, pulse: &mut dyn Write) -> Result<Value>;
    fn exec(&mut self, code: &str) -> Result<Execution>;
    fn steer(&mut self) -> Result<Vec<String>>;
}

pub struct Live<'a> {
    pub client: &'a mut Client,
    pub python: &'a mut Py,
    pub intake: &'a mut Option<Intake>,
    pub cancelled: Option<&'a AtomicBool>,
}

impl Ports for Live<'_> {
    fn post(&mut self, request: &dyn Transcript, pulse: &mut dyn Write) -> Result<Value> {
        refuse_if_cancelled(self.cancelled)?;
        request.post(self.client, pulse)
    }

    fn exec(&mut self, code: &str) -> Result<Execution> {
        refuse_if_cancelled(self.cancelled)?;
        self.python.run_observed(code)
    }

    fn steer(&mut self) -> Result<Vec<String>> {
        match self.intake.as_mut() {
            Some(intake) => intake.drain(),
            None => Ok(Vec::new()),
        }
    }
}

fn refuse_if_cancelled(cancelled: Option<&AtomicBool>) -> Result<()> {
    if cancelled.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
        return Err(protocol(
            "continue an ACP turn",
            "session/cancel arrived before the next model or tool step",
            "wait for stopReason cancelled; start a new prompt if the task should continue",
        )
        .into());
    }
    Ok(())
}

pub fn drive(request: &mut Value, live: &mut Live<'_>, trace: &mut dyn Write) -> Result<String> {
    buffered_run(request, live, trace)
}

pub fn turn(
    request: &mut Request,
    prompt: Value,
    live: &mut Live<'_>,
    trace: &mut dyn Write,
) -> Result<String> {
    request.history.push(&Value::Object(Map::from_iter([
        ("role".to_owned(), Value::from("user")),
        ("content".to_owned(), prompt),
    ])))?;
    buffered_run(request, live, trace)
}

fn buffered_run(
    request: &mut dyn Transcript,
    live: &mut Live<'_>,
    trace: &mut dyn Write,
) -> Result<String> {
    let mut buffered = BufWriter::new(trace);
    let result = run(request, live, &mut buffered);
    drop(buffered.into_parts());
    result
}

fn found(value: Option<&Value>) -> String {
    value.map_or_else(|| "<missing>".to_owned(), |value| value.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Call<'a> {
    id: &'a str,
    code: &'a str,
}

pub(crate) fn run(
    request: &mut dyn Transcript,
    ports: &mut dyn Ports,
    trace: &mut dyn Write,
) -> Result<String> {
    let mut first_response = true;
    loop {
        let mut response = ports.post(request, trace)?;
        if let Some(usage) = response.get("usage") {
            observation::write(
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
        let content = response
            .get_mut("content")
            .map(Value::take)
            .ok_or_else(|| {
                protocol(
                    "read response.content",
                    "content is missing",
                    "return a complete Anthropic Messages response with content",
                )
                .with_path("response.content")
                .with_value("<missing>")
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
            request.trace_prompt(trace)?;
            first_response = false;
        }
        let (index, assistant) = append(request, content, Some(&stop), trace)?;
        let content = &assistant.as_ref()["content"];
        let base = format!("messages[{index}]");
        match stop.as_str() {
            "pause_turn" => continue,
            "end_turn" => return text(content, &base),
            "tool_use" => {
                let calls = calls(content, &base)?;
                if calls.is_empty() {
                    return Err(protocol(
                        "validate a python tool wave",
                        "stop_reason is tool_use but no client tool call is present",
                        "return at least one valid python tool_use block, or use another stop_reason",
                    )
                    .with_path(format!("{base}.content"))
                    .with_value(found(Some(content)))
                    .into());
                }
                let mut results = Vec::with_capacity(calls.len());
                let mut previews = Vec::new();
                for call in calls {
                    observation::write(
                        trace,
                        &json!({"type": "mei.obs", "kind": "tool_start", "id": call.id}),
                        "trace the started Python call",
                    )?;
                    let output = ports.exec(call.code)?;
                    previews.extend(output.previews);
                    let mut result = Map::from_iter([
                        ("type".to_owned(), Value::from("tool_result")),
                        ("tool_use_id".to_owned(), Value::from(call.id)),
                        ("content".to_owned(), output.content),
                    ]);
                    if output.failed {
                        result.insert("is_error".to_owned(), Value::Bool(true));
                    }
                    results.push(Value::Object(result));
                }
                let arrived = ports.steer()?;
                steer::land(&mut results, &arrived);
                append(request, Value::Array(results), None, trace)?;
                for preview in previews {
                    observation::write(
                        trace,
                        &json!({"type": "mei.obs", "kind": "image", "preview": preview}),
                        "trace an image preview",
                    )?;
                }
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

fn append<'a>(
    request: &'a mut dyn Transcript,
    content: Value,
    stop: Option<&str>,
    trace: &mut dyn Write,
) -> Result<(usize, Cow<'a, Value>)> {
    let role = stop.map_or("user", |_| "assistant");
    let fields = Map::from_iter([
        ("role".to_owned(), Value::from(role)),
        ("content".to_owned(), content),
    ]);
    request.store(fields, stop, trace)
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

fn calls<'a>(content: &'a Value, base: &str) -> Result<Vec<Call<'a>>> {
    let mut calls = Vec::new();
    let mut ids = HashSet::new();
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
        if !ids.insert(id) {
            return Err(protocol(
                "validate a python tool wave",
                format!("duplicate tool_use id `{id}`"),
                "give every tool_use in this wave a unique id; no call in the wave was executed",
            )
            .with_path(format!("{base}.content[{index}].id"))
            .with_value(id)
            .into());
        }
        calls.push(Call { id, code });
    }
    Ok(calls)
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
