// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Anthropic Messages wire, in both directions.
//!
//! This is the canonical shape's own dialect, so translation here is
//! mostly transcription — which is exactly why the losses are somewhere
//! else and this module carries none.
//!
//! **Read the provider's own documentation before changing anything
//! here.** Every field is the provider's shape, not ours; a shape that
//! looks wrong is usually a shape that changed.
//!
//! - Request and response: <https://platform.claude.com/docs/en/api/messages>
//! - Thinking blocks, `signature`, preservation across tool use:
//!   <https://platform.claude.com/docs/en/build-with-claude/thinking>
//! - Effort levels: <https://platform.claude.com/docs/en/build-with-claude/effort>
//! - What invalidates a cache breakpoint:
//!   <https://platform.claude.com/docs/en/build-with-claude/prompt-caching>
//! - The 400 that a modified thinking block earns:
//!   <https://platform.claude.com/docs/en/agents-and-tools/tool-use/troubleshooting-tool-use>
//! - Pictures, and the base64 source block:
//!   <https://platform.claude.com/docs/en/build-with-claude/vision>

use kernel::{
    AxError, ChatRequest, ChatResponse, ContentBlock, Effort, ModelUsage, Role, StopReason,
};
use serde_json::{Map, Value, json};

use crate::dialect::ImageBytes;
use crate::mismatch::{
    as_str, mismatch, mismatch_found, payload_from, require, tokens_or_zero, unspelled_effort,
};

mod stream;

pub(crate) use stream::{increment_of, settled};

fn role_str(role: Role) -> Result<&'static str, AxError> {
    match role {
        Role::User => Ok("user"),
        Role::Assistant => Ok("assistant"),
        _ => Err(mismatch("message.role", "unknown canonical role")),
    }
}

/// One picture, as this wire spells it.
fn image_wire(picture: &kernel::ImageRef, images: &ImageBytes) -> Result<Value, AxError> {
    Ok(json!({
        "type": "image",
        "source": {
            "type": "base64",
            "media_type": picture.media_type.mime(),
            "data": images.encoded(&picture.locator)?,
        },
    }))
}

fn block_wire(block: &ContentBlock, images: &ImageBytes) -> Result<Value, AxError> {
    match block {
        ContentBlock::Text { text } => Ok(json!({ "type": "text", "text": text })),
        // Verbatim in both directions: the provider verifies `signature`
        // against the reasoning it issued, and a modified thinking block
        // is refused with a 400 rather than tolerated.
        ContentBlock::Thinking {
            thinking,
            signature,
        } => Ok(json!({ "type": "thinking", "thinking": thinking, "signature": signature })),
        ContentBlock::RedactedThinking { data } => {
            Ok(json!({ "type": "redacted_thinking", "data": data }))
        }
        ContentBlock::ToolUse { id, name, input } => Ok(json!({
            "type": "tool_use", "id": id, "name": name.as_str(),
            "input": serde_json::to_value(input)
                .map_err(|err| mismatch("tool_use.input", &err.to_string()))?,
        })),
        ContentBlock::Image(picture) => image_wire(picture, images),
        // A tool result with pictures spells its content as blocks; one
        // without keeps the string form, so every request that carried
        // no picture crosses byte for byte as it did before.
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
            attachments,
        } => {
            let carried = if attachments.is_empty() {
                Value::String(content.clone())
            } else {
                let mut parts = vec![json!({ "type": "text", "text": content })];
                for picture in attachments {
                    parts.push(image_wire(picture, images)?);
                }
                Value::Array(parts)
            };
            Ok(json!({
                "type": "tool_result", "tool_use_id": tool_use_id,
                "content": carried, "is_error": is_error,
            }))
        }
        _ => Err(mismatch("content.block", "unknown canonical block kind")),
    }
}

pub(crate) fn request(req: &ChatRequest, images: &ImageBytes) -> Result<Value, AxError> {
    let mut root = Map::new();
    root.insert("model".to_owned(), Value::String(req.model.clone()));
    // This wire requires the field, so a model whose ceiling nobody
    // registered is refused here rather than sent a number chosen in
    // this file: a ceiling invented at the call site truncates runs for
    // a reason that appears nowhere in the account.
    let ceiling = req.max_tokens.ok_or_else(|| {
        AxError::failure(
            kernel::AxCode::ConfigInvalid,
            "write an Anthropic request",
            format!("{} states no output ceiling", req.model),
        )
        .with_recovery("state this model's max output tokens where the model is registered")
    })?;
    root.insert("max_tokens".to_owned(), Value::Number(ceiling.get().into()));
    if !req.system.is_empty() {
        let system: Vec<Value> = req
            .system
            .iter()
            .map(|block| {
                let mut entry = Map::new();
                entry.insert("type".to_owned(), Value::String("text".to_owned()));
                entry.insert("text".to_owned(), Value::String(block.text.clone()));
                if block.cache {
                    entry.insert("cache_control".to_owned(), json!({ "type": "ephemeral" }));
                }
                Value::Object(entry)
            })
            .collect();
        root.insert("system".to_owned(), Value::Array(system));
    }
    let mut messages = Vec::new();
    for message in &req.messages {
        let role = role_str(message.role)?;
        let blocks: Result<Vec<Value>, AxError> = message
            .content
            .iter()
            .map(|block| block_wire(block, images))
            .collect();
        messages.push(json!({ "role": role, "content": blocks? }));
    }
    root.insert("messages".to_owned(), Value::Array(messages));
    for (key, value) in effort_fields(req.effort)? {
        root.insert(key.to_owned(), value);
    }
    if !req.tools.is_empty() {
        let tools: Result<Vec<Value>, AxError> = req
            .tools
            .iter()
            .map(|tool| {
                Ok(json!({
                    "name": tool.name.as_str(),
                    "description": tool.description,
                    "input_schema": serde_json::to_value(&tool.input_schema)
                        .map_err(|err| mismatch("tools.input_schema", &err.to_string()))?,
                }))
            })
            .collect();
        root.insert("tools".to_owned(), Value::Array(tools?));
    }
    Ok(Value::Object(root))
}

fn block_from(value: &Value, path: &str) -> Result<ContentBlock, AxError> {
    let kind = as_str(require(value, path, "type")?, &format!("{path}.type"))?;
    match kind {
        "text" => Ok(ContentBlock::Text {
            text: as_str(require(value, path, "text")?, &format!("{path}.text"))?.to_owned(),
        }),
        "thinking" => Ok(ContentBlock::Thinking {
            thinking: as_str(
                require(value, path, "thinking")?,
                &format!("{path}.thinking"),
            )?
            .to_owned(),
            signature: as_str(
                require(value, path, "signature")?,
                &format!("{path}.signature"),
            )?
            .to_owned(),
        }),
        "redacted_thinking" => Ok(ContentBlock::RedactedThinking {
            data: as_str(require(value, path, "data")?, &format!("{path}.data"))?.to_owned(),
        }),
        "tool_use" => {
            let name_raw = as_str(require(value, path, "name")?, &format!("{path}.name"))?;
            let name = kernel::ToolName::parse(name_raw).map_err(|_| {
                mismatch_found(&format!("{path}.name"), "not a tool name", name_raw)
            })?;
            Ok(ContentBlock::ToolUse {
                id: as_str(require(value, path, "id")?, &format!("{path}.id"))?.to_owned(),
                name,
                input: payload_from(require(value, path, "input")?, &format!("{path}.input"))?,
            })
        }
        "tool_result" => Ok(ContentBlock::ToolResult {
            tool_use_id: as_str(
                require(value, path, "tool_use_id")?,
                &format!("{path}.tool_use_id"),
            )?
            .to_owned(),
            content: as_str(require(value, path, "content")?, &format!("{path}.content"))?
                .to_owned(),
            is_error: value
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            // This provider answers with assistant content, never with
            // a tool result it produced, so nothing arrives here to
            // attach; inventing one would be a picture nobody sent.
            attachments: Vec::new(),
        }),
        other => Err(mismatch(&format!("{path}.type"), other)),
    }
}

fn stop_from(raw: &str, path: &str) -> Result<StopReason, AxError> {
    match raw {
        "end_turn" => Ok(StopReason::EndTurn),
        "tool_use" => Ok(StopReason::ToolUse),
        "max_tokens" => Ok(StopReason::MaxTokens),
        other => Err(mismatch(path, other)),
    }
}

pub(crate) fn response_from(wire: &Value) -> Result<ChatResponse, AxError> {
    let content_value = require(wire, "response", "content")?;
    let items = content_value
        .as_array()
        .ok_or_else(|| mismatch("response.content", "expected array"))?;
    let mut content = Vec::new();
    for (i, item) in items.iter().enumerate() {
        content.push(block_from(item, &format!("response.content[{i}]"))?);
    }
    let stop = stop_from(
        as_str(
            require(wire, "response", "stop_reason")?,
            "response.stop_reason",
        )?,
        "response.stop_reason",
    )?;
    let usage_value = require(wire, "response", "usage")?;
    let usage = ModelUsage {
        input_tokens: tokens_or_zero(usage_value, "input_tokens", "response.usage")?,
        output_tokens: tokens_or_zero(usage_value, "output_tokens", "response.usage")?,
        cache_read_tokens: tokens_or_zero(
            usage_value,
            "cache_read_input_tokens",
            "response.usage",
        )?,
        cache_write_tokens: tokens_or_zero(
            usage_value,
            "cache_creation_input_tokens",
            "response.usage",
        )?,
    };
    Ok(ChatResponse {
        content,
        stop,
        usage,
    })
}

fn stop_str(stop: StopReason) -> Result<&'static str, AxError> {
    match stop {
        StopReason::EndTurn => Ok("end_turn"),
        StopReason::ToolUse => Ok("tool_use"),
        StopReason::MaxTokens => Ok("max_tokens"),
        _ => Err(mismatch("stop_reason", "unknown canonical stop reason")),
    }
}

/// The response direction carries no pictures: a provider answers with
/// text, thinking and tool calls, so there is nothing to resolve.
pub(crate) fn response_wire(resp: &ChatResponse) -> Result<Value, AxError> {
    let empty = ImageBytes::default();
    let content: Result<Vec<Value>, AxError> = resp
        .content
        .iter()
        .map(|block| block_wire(block, &empty))
        .collect();
    Ok(json!({
        "content": content?,
        "stop_reason": stop_str(resp.stop)?,
        "usage": {
            "input_tokens": resp.usage.input_tokens.get(),
            "output_tokens": resp.usage.output_tokens.get(),
            "cache_read_input_tokens": resp.usage.cache_read_tokens.get(),
            "cache_creation_input_tokens": resp.usage.cache_write_tokens.get(),
        },
    }))
}

/// The request field that states how hard to think. This dialect spells
/// the five working levels in `effort`, and spells "do not think" in a
/// different field entirely — `effort` has no `none`.
fn effort_fields(effort: Option<Effort>) -> Result<Vec<(&'static str, Value)>, AxError> {
    let Some(effort) = effort else {
        return Ok(Vec::new());
    };
    let level = match effort {
        Effort::None => return Ok(vec![("thinking", json!({ "type": "disabled" }))]),
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
        Effort::XHigh => "xhigh",
        Effort::Max => "max",
        _ => return Err(unspelled_effort(effort, "anthropic")),
    };
    Ok(vec![("effort", Value::String(level.to_owned()))])
}
