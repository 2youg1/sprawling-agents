// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    AxCode, AxError, CacheCount, ChatRequest, ChatResponse, ContentBlock, DialectKind, ModelUsage,
    Role, StopReason,
};
use serde_json::{Map, Value, json};

use crate::dialect::ImageBytes;
use crate::mismatch::{
    as_str, cache_count, mismatch, mismatch_found, payload_from, require, tokens_or_zero,
};
use crate::provider::preset::{CeilingField, ChatSpelling, ReasoningReturn, ToolPictureCarry};

mod stream;

pub(crate) use stream::{increment_of, settled};

fn empty_answer() -> AxError {
    AxError::failure(
        AxCode::Provider,
        "read the provider's answer",
        "the provider accepted the request and returned no answer at all",
    )
    .retriable()
    .with_recovery(
        "lower this model's max output tokens - a ceiling above what the model allows is \
         answered this way rather than refused - then dispatch again",
    )
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

fn reasoning_returned(content: &[ContentBlock], reasoning: ReasoningReturn) -> Option<String> {
    match reasoning {
        ReasoningReturn::Dropped => None,
        ReasoningReturn::AsReasoningContent => {
            let thought: Vec<&str> = content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Thinking { thinking, .. } => Some(thinking.as_str()),
                    ContentBlock::Text { .. }
                    | ContentBlock::RedactedThinking { .. }
                    | ContentBlock::ToolUse { .. }
                    | ContentBlock::ToolResult { .. }
                    | ContentBlock::Image(_) => None,
                })
                .collect();
            (!thought.is_empty()).then(|| thought.join("\n\n"))
        }
    }
}

fn image_part(picture: &kernel::ImageRef, images: &ImageBytes) -> Result<Value, AxError> {
    let data = images.encoded(&picture.locator)?;
    Ok(json!({
        "type": "image_url",
        "image_url": { "url": format!("data:{};base64,{data}", picture.media_type.mime()) },
    }))
}

fn pictures_following(
    content: &[ContentBlock],
    images: &ImageBytes,
    carry: ToolPictureCarry,
) -> Result<Vec<Value>, AxError> {
    let mut parts = Vec::new();
    for block in content {
        match block {
            ContentBlock::Image(picture) => parts.push(image_part(picture, images)?),
            ContentBlock::ToolResult { attachments, .. }
                if carry == ToolPictureCarry::FollowingUser =>
            {
                for picture in attachments {
                    parts.push(image_part(picture, images)?);
                }
            }
            ContentBlock::ToolResult { .. }
            | ContentBlock::Text { .. }
            | ContentBlock::Thinking { .. }
            | ContentBlock::RedactedThinking { .. }
            | ContentBlock::ToolUse { .. } => {}
        }
    }
    Ok(parts)
}

fn tool_content(
    content: &str,
    attachments: &[kernel::ImageRef],
    images: &ImageBytes,
    carry: ToolPictureCarry,
) -> Result<Value, AxError> {
    if attachments.is_empty() || carry == ToolPictureCarry::FollowingUser {
        return Ok(Value::String(content.to_owned()));
    }
    let mut parts = vec![json!({ "type": "text", "text": content })];
    for picture in attachments {
        parts.push(image_part(picture, images)?);
    }
    Ok(Value::Array(parts))
}

pub(crate) fn request(
    req: &ChatRequest,
    images: &ImageBytes,
    spelling: ChatSpelling,
) -> Result<Value, AxError> {
    let mut messages = Vec::new();
    if !req.system.is_empty() {

        let system: Vec<&str> = req.system.iter().map(|b| b.text.as_str()).collect();
        messages.push(json!({ "role": "system", "content": system.join("\n\n") }));
    }
    for message in req.messages.iter() {
        match message.role {
            Role::User => {
                for block in &message.content {
                    if let ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error: _,
                        attachments,
                    } = block
                    {
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_use_id,
                            "content": tool_content(
                                content,
                                attachments,
                                images,
                                spelling.chat_tool_pictures,
                            )?,
                        }));
                    }
                }

                let text = joined_text(&message.content);
                let pictures =
                    pictures_following(&message.content, images, spelling.chat_tool_pictures)?;
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
                if let Some(thinking) = reasoning_returned(&message.content, spelling.reasoning) {
                    entry.insert("reasoning_content".to_owned(), Value::String(thinking));
                }
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
        }
    }
    let mut root = Map::new();
    root.insert("model".to_owned(), Value::String(req.model.clone()));

    if let Some(ceiling) = req.max_tokens {
        let field = match spelling.ceiling {
            CeilingField::MaxTokens => "max_tokens",
            CeilingField::MaxCompletionTokens => "max_completion_tokens",
        };
        root.insert(field.to_owned(), Value::Number(ceiling.get().into()));
    }
    root.insert("messages".to_owned(), Value::Array(messages));
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
    let cache_read = cache_count(
        usage_value.get("prompt_tokens_details"),
        "cached_tokens",
        "response.usage.prompt_tokens_details",
    )?;
    let usage = ModelUsage {
        input_tokens: tokens_or_zero(usage_value, "prompt_tokens", "response.usage")?,
        output_tokens: tokens_or_zero(usage_value, "completion_tokens", "response.usage")?,
        cache_read_tokens: cache_read,

        cache_write_tokens: CacheCount::Unreported,
        dialect: Some(DialectKind::OpenAi),
    };
    Ok(ChatResponse {
        content,
        stop,
        usage,
    })
}

#[cfg(test)]
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
    };
    Ok(json!({
        "choices": [ { "message": Value::Object(message), "finish_reason": finish } ],
        "usage": {
            "prompt_tokens": resp.usage.input_tokens.get(),
            "completion_tokens": resp.usage.output_tokens.get(),
            "prompt_tokens_details": { "cached_tokens": resp.usage.cache_read_tokens.reported() },
        },
    }))
}
