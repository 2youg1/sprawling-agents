// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    AxError, CacheCount, ChatRequest, ChatResponse, ContentBlock, DialectKind, ModelUsage, Role,
    StopReason, Tokens,
};
use serde_json::{Map, Value, json};

use crate::dialect::ImageBytes;
use crate::mismatch::{
    as_str, cache_count, mismatch, mismatch_found, payload_from, require, tokens_or_zero,
};

mod stream;

pub(crate) use stream::{call_completed_by, increment_of, settled};

fn role_str(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

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
    }
}

pub(crate) fn request(req: &ChatRequest, images: &ImageBytes) -> Result<Value, AxError> {
    let mut root = Map::new();
    root.insert("model".to_owned(), Value::String(req.model.clone()));

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
    for (index, message) in req.messages.iter().enumerate() {
        let role = role_str(message.role);
        let mut blocks = message
            .content
            .iter()
            .map(|block| block_wire(block, images))
            .collect::<Result<Vec<Value>, AxError>>()?;

        if req.carries_breakpoint(index)
            && let Some(Value::Object(last)) = blocks.last_mut()
        {
            last.insert("cache_control".to_owned(), json!({ "type": "ephemeral" }));
        }
        messages.push(json!({ "role": role, "content": blocks }));
    }
    root.insert("messages".to_owned(), Value::Array(messages));
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
        "thinking" => {
            let signature = as_str(
                require(value, path, "signature")?,
                &format!("{path}.signature"),
            )?;

            if signature.is_empty() {
                return Err(mismatch(
                    &format!("{path}.signature"),
                    "empty; the thinking block arrived without the signature the provider \
                     verifies it against",
                ));
            }
            Ok(ContentBlock::Thinking {
                thinking: as_str(
                    require(value, path, "thinking")?,
                    &format!("{path}.thinking"),
                )?
                .to_owned(),
                signature: signature.to_owned(),
            })
        }
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
    let cache_read_tokens = cache_count(
        Some(usage_value),
        "cache_read_input_tokens",
        "response.usage",
    )?;
    let cache_write_tokens = cache_count(
        Some(usage_value),
        "cache_creation_input_tokens",
        "response.usage",
    )?;

    let input_tokens = [cache_read_tokens, cache_write_tokens]
        .into_iter()
        .map(CacheCount::or_zero)
        .try_fold(
            tokens_or_zero(usage_value, "input_tokens", "response.usage")?,
            Tokens::checked_add,
        )
        .ok_or_else(|| mismatch("response.usage", "input counts that overflow a token count"))?;
    let usage = ModelUsage {
        input_tokens,
        output_tokens: tokens_or_zero(usage_value, "output_tokens", "response.usage")?,
        cache_read_tokens,
        cache_write_tokens,
        dialect: Some(DialectKind::Anthropic),
    };
    Ok(ChatResponse {
        content,
        stop,
        usage,
    })
}

#[cfg(test)]
fn stop_str(stop: StopReason) -> &'static str {
    match stop {
        StopReason::EndTurn => "end_turn",
        StopReason::ToolUse => "tool_use",
        StopReason::MaxTokens => "max_tokens",
    }
}

#[cfg(test)]
pub(crate) fn response_wire(resp: &ChatResponse) -> Result<Value, AxError> {
    let empty = ImageBytes::default();
    let content: Result<Vec<Value>, AxError> = resp
        .content
        .iter()
        .map(|block| block_wire(block, &empty))
        .collect();
    Ok(json!({
        "content": content?,
        "stop_reason": stop_str(resp.stop),
        "usage": {
            "input_tokens": resp.usage.input_tokens.get()
                .saturating_sub(resp.usage.cache_read_tokens.or_zero().get())
                .saturating_sub(resp.usage.cache_write_tokens.or_zero().get()),
            "output_tokens": resp.usage.output_tokens.get(),
            "cache_read_input_tokens": resp.usage.cache_read_tokens.reported(),
            "cache_creation_input_tokens": resp.usage.cache_write_tokens.reported(),
        },
    }))
}
