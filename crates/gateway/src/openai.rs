// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The OpenAI Chat Completions wire, in both directions, and every loss
//! the translation takes.
//!
//! **Read the provider's own documentation before changing anything
//! here.** Every field is the provider's shape, not ours; a shape that
//! looks wrong is usually a shape that changed.
//!
//! - Request and response:
//!   <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>
//! - `reasoning.effort` and its accepted values:
//!   <https://developers.openai.com/api/docs/guides/reasoning>
//!
//! **Loss accounting is explicit, because this dialect is not the
//! canonical shape.** This wire has no explicit cache breakpoints
//! (provider caching is implicit prefix-matching), so `cache` markers
//! drop on this path; it cannot spell a thinking block, so one is not
//! sent rather than sent as prose; and its `tool` message accepts only
//! a string, so pictures a tool produced ride in the user message that
//! immediately follows it — the pictures arrive, and what is lost is
//! the statement that they came out of that tool call. All three are
//! asserted in tests.

use kernel::{
    AxCode, AxError, ChatRequest, ChatResponse, ContentBlock, Effort, ModelUsage, Role, StopReason,
    Tokens,
};
use serde_json::{Map, Value, json};

use crate::dialect::ImageBytes;
use crate::mismatch::{
    as_str, mismatch, mismatch_found, payload_from, require, tokens_or_zero, unspelled_effort,
};

mod stream;

pub(crate) use stream::{increment_of, settled};

/// The provider took the request, answered 200, and put nothing in it.
///
/// Told apart from every other shape mismatch because it is not one: the
/// envelope is this dialect's, every field is where it belongs, and
/// `choices` is null. Observed on a hosted OpenAI-compatible endpoint
/// when `max_tokens` is above what the chosen model allows - the request
/// is neither refused nor answered, it is dropped, and the only trace is
/// a nulled answer with a zeroed usage block. A person reading
/// "expected array" learns nothing they can act on, which is why this
/// case says what to change.
fn empty_answer() -> AxError {
    AxError::failure(
        AxCode::Provider,
        "read the provider's answer",
        "the provider accepted the request and returned no answer at all",
    )
    .with_recovery(
        "lower this model's max output tokens - a ceiling above what the model allows is \
         answered this way rather than refused - then dispatch again",
    )
    .retriable()
}

/// This dialect writes every level in one field, `none` included.
fn effort_field(effort: Effort) -> Result<&'static str, AxError> {
    match effort {
        Effort::None => Ok("none"),
        Effort::Low => Ok("low"),
        Effort::Medium => Ok("medium"),
        Effort::High => Ok("high"),
        Effort::XHigh => Ok("xhigh"),
        Effort::Max => Ok("max"),
        _ => Err(unspelled_effort(effort, "openai")),
    }
}

fn joined_text(content: &[ContentBlock]) -> String {
    let mut out = String::new();
    for block in content {
        if let ContentBlock::Text { text } = block {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(text);
        }
    }
    out
}

/// One picture, as this wire spells it: a data URL inside a content
/// part, which is the only place this dialect accepts image bytes.
fn image_part(picture: &kernel::ImageRef, images: &ImageBytes) -> Result<Value, AxError> {
    let data = images.encoded(&picture.locator)?;
    Ok(json!({
        "type": "image_url",
        "image_url": { "url": format!("data:{};base64,{data}", picture.media_type.mime()) },
    }))
}

/// Every picture one user message refers to, in the order the blocks
/// name them: the message's own `Image` blocks first, then whatever its
/// tool results attached.
fn pictures_of(content: &[ContentBlock], images: &ImageBytes) -> Result<Vec<Value>, AxError> {
    let mut parts = Vec::new();
    for block in content {
        match block {
            ContentBlock::Image(picture) => parts.push(image_part(picture, images)?),
            ContentBlock::ToolResult { attachments, .. } => {
                for picture in attachments {
                    parts.push(image_part(picture, images)?);
                }
            }
            _ => {}
        }
    }
    Ok(parts)
}

pub(crate) fn request(req: &ChatRequest, images: &ImageBytes) -> Result<Value, AxError> {
    let mut messages = Vec::new();
    if !req.system.is_empty() {
        // Explicit breakpoints have no OpenAI wire slot; the marker drops
        // here by design (provider caching is implicit prefix matching).
        let system: Vec<&str> = req.system.iter().map(|b| b.text.as_str()).collect();
        messages.push(json!({ "role": "system", "content": system.join("\n\n") }));
    }
    for message in &req.messages {
        match message.role {
            Role::User => {
                for block in &message.content {
                    if let ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error: _,
                        attachments: _,
                    } = block
                    {
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_use_id, "content": content,
                        }));
                    }
                }
                // Pictures cannot ride a `tool` message on this wire, so
                // they land in the user message that follows it. The
                // loss is which tool call produced them, and it is
                // recorded at the top of this file rather than taken
                // silently.
                let text = joined_text(&message.content);
                let pictures = pictures_of(&message.content, images)?;
                if pictures.is_empty() {
                    if !text.is_empty() {
                        messages.push(json!({ "role": "user", "content": text }));
                    }
                } else {
                    let mut parts = Vec::new();
                    if !text.is_empty() {
                        parts.push(json!({ "type": "text", "text": text }));
                    }
                    parts.extend(pictures);
                    messages.push(json!({ "role": "user", "content": parts }));
                }
            }
            Role::Assistant => {
                let text = joined_text(&message.content);
                let mut entry = Map::new();
                entry.insert("role".to_owned(), Value::String("assistant".to_owned()));
                entry.insert(
                    "content".to_owned(),
                    if text.is_empty() {
                        Value::Null
                    } else {
                        Value::String(text)
                    },
                );
                let mut tool_calls = Vec::new();
                for block in &message.content {
                    if let ContentBlock::ToolUse { id, name, input } = block {
                        let arguments = serde_json::to_string(input)
                            .map_err(|err| mismatch("tool_use.input", &err.to_string()))?;
                        tool_calls.push(json!({
                            "id": id, "type": "function",
                            "function": { "name": name.as_str(), "arguments": arguments },
                        }));
                    }
                }
                if !tool_calls.is_empty() {
                    entry.insert("tool_calls".to_owned(), Value::Array(tool_calls));
                }
                messages.push(Value::Object(entry));
            }
            _ => return Err(mismatch("message.role", "unknown canonical role")),
        }
    }
    let mut root = Map::new();
    root.insert("model".to_owned(), Value::String(req.model.clone()));
    // Absent when no catalogue row states this model's ceiling: this
    // wire permits the field to be missing, and the provider's own
    // default is a real number, where a zero is a reply with nothing in
    // it that reads downstream as work that finished.
    if let Some(ceiling) = req.max_tokens {
        root.insert("max_tokens".to_owned(), Value::Number(ceiling.get().into()));
    }
    root.insert("messages".to_owned(), Value::Array(messages));
    if let Some(effort) = req.effort {
        root.insert(
            "reasoning".to_owned(),
            json!({ "effort": effort_field(effort)? }),
        );
    }
    if !req.tools.is_empty() {
        let tools: Result<Vec<Value>, AxError> = req
            .tools
            .iter()
            .map(|tool| {
                Ok(json!({
                    "type": "function",
                    "function": {
                        "name": tool.name.as_str(),
                        "description": tool.description,
                        "parameters": serde_json::to_value(&tool.input_schema)
                            .map_err(|err| mismatch("tools.parameters", &err.to_string()))?,
                    },
                }))
            })
            .collect();
        root.insert("tools".to_owned(), Value::Array(tools?));
    }
    Ok(Value::Object(root))
}

pub(crate) fn response_from(wire: &Value) -> Result<ChatResponse, AxError> {
    let offered = require(wire, "response", "choices")?;
    // A null here is a provider that dropped the request rather than a
    // provider speaking a shape we do not know.
    if offered.is_null() {
        return Err(empty_answer());
    }
    let choices = offered
        .as_array()
        .ok_or_else(|| mismatch("response.choices", "expected array"))?;
    let first = choices
        .first()
        .ok_or_else(|| mismatch("response.choices", "empty"))?;
    let message = require(first, "response.choices[0]", "message")?;
    let mut content = Vec::new();
    // First, because it is what the model did first, and because a
    // page that folds it needs it to sit above the answer it produced.
    // The signature is empty: this wire has none to carry, and an
    // invented one would be sent back to a provider that checks it.
    if let Some(thinking) = message.as_object().and_then(stream::reasoning_in) {
        content.push(ContentBlock::Thinking {
            thinking,
            signature: String::new(),
        });
    }
    if let Some(text) = message.get("content").and_then(Value::as_str)
        && !text.is_empty()
    {
        content.push(ContentBlock::Text {
            text: text.to_owned(),
        });
    }
    if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
        for (i, call) in calls.iter().enumerate() {
            let path = format!("response.choices[0].message.tool_calls[{i}]");
            let function = require(call, &path, "function")?;
            let name_raw = as_str(require(function, &path, "name")?, &format!("{path}.name"))?;
            let name = kernel::ToolName::parse(name_raw).map_err(|_| {
                mismatch_found(&format!("{path}.name"), "not a tool name", name_raw)
            })?;
            let arguments = as_str(
                require(function, &path, "arguments")?,
                &format!("{path}.arguments"),
            )?;
            let parsed: Value = serde_json::from_str(arguments)
                .map_err(|err| mismatch(&format!("{path}.arguments"), &err.to_string()))?;
            content.push(ContentBlock::ToolUse {
                id: as_str(require(call, &path, "id")?, &format!("{path}.id"))?.to_owned(),
                name,
                input: payload_from(&parsed, &format!("{path}.arguments"))?,
            });
        }
    }
    let stop = match as_str(
        require(first, "response.choices[0]", "finish_reason")?,
        "response.choices[0].finish_reason",
    )? {
        "stop" => StopReason::EndTurn,
        "tool_calls" => StopReason::ToolUse,
        "length" => StopReason::MaxTokens,
        other => return Err(mismatch("response.choices[0].finish_reason", other)),
    };
    let usage_value = require(wire, "response", "usage")?;
    let cache_read = usage_value
        .get("prompt_tokens_details")
        .map(|details| {
            tokens_or_zero(
                details,
                "cached_tokens",
                "response.usage.prompt_tokens_details",
            )
        })
        .transpose()?
        .unwrap_or(Tokens::new(0));
    let usage = ModelUsage {
        input_tokens: tokens_or_zero(usage_value, "prompt_tokens", "response.usage")?,
        output_tokens: tokens_or_zero(usage_value, "completion_tokens", "response.usage")?,
        cache_read_tokens: cache_read,
        // No OpenAI wire slot: cache writes are not reported distinctly.
        cache_write_tokens: Tokens::new(0),
    };
    Ok(ChatResponse {
        content,
        stop,
        usage,
    })
}

pub(crate) fn response_wire(resp: &ChatResponse) -> Result<Value, AxError> {
    let text = joined_text(&resp.content);
    let mut message = Map::new();
    message.insert("role".to_owned(), Value::String("assistant".to_owned()));
    message.insert(
        "content".to_owned(),
        if text.is_empty() {
            Value::Null
        } else {
            Value::String(text)
        },
    );
    let mut tool_calls = Vec::new();
    for block in &resp.content {
        if let ContentBlock::ToolUse { id, name, input } = block {
            let arguments = serde_json::to_string(input)
                .map_err(|err| mismatch("tool_use.input", &err.to_string()))?;
            tool_calls.push(json!({
                "id": id, "type": "function",
                "function": { "name": name.as_str(), "arguments": arguments },
            }));
        }
    }
    if !tool_calls.is_empty() {
        message.insert("tool_calls".to_owned(), Value::Array(tool_calls));
    }
    let finish = match resp.stop {
        StopReason::EndTurn => "stop",
        StopReason::ToolUse => "tool_calls",
        StopReason::MaxTokens => "length",
        _ => return Err(mismatch("stop_reason", "unknown canonical stop reason")),
    };
    Ok(json!({
        "choices": [ { "message": Value::Object(message), "finish_reason": finish } ],
        "usage": {
            "prompt_tokens": resp.usage.input_tokens.get(),
            "completion_tokens": resp.usage.output_tokens.get(),
            "prompt_tokens_details": { "cached_tokens": resp.usage.cache_read_tokens.get() },
        },
    }))
}
