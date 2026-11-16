// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;
use kernel::{ContentBlock, Increment, ToolCall};
use serde_json::{Value, json};

use crate::endpoint::failure::{ProviderFailure, provider_err};
use crate::mismatch::{settled_tool_arguments, stream_cut};

pub(crate) fn increment_of(map: &serde_json::Map<String, Value>) -> Option<Increment> {
    let delta = map.get("delta")?.as_object()?;
    match delta.get("type")?.as_str()? {
        "text_delta" => Some(Increment::Said(delta.get("text")?.as_str()?.to_owned())),
        "thinking_delta" => Some(Increment::Thought(
            delta.get("thinking")?.as_str()?.to_owned(),
        )),
        _ => None,
    }
}

pub(crate) fn settled(frames: &[Value]) -> Result<Value, AxError> {
    let Rebuilt {
        content,
        usage,
        stop,
    } = rebuilt(frames.iter())?;
    let Some(stop) = stop else {
        return Err(stream_cut(
            "the stream ended without the frame that says why the model stopped",
        ));
    };
    Ok(json!({ "content": content, "stop_reason": stop, "usage": usage }))
}

pub(crate) fn call_completed_by(frames: &[Value]) -> Result<Option<ToolCall>, AxError> {
    let stopped = frames
        .last()
        .filter(|frame| frame.get("type").and_then(Value::as_str) == Some("content_block_stop"))
        .and_then(|frame| frame.get("index"))
        .and_then(Value::as_u64);
    match stopped {
        Some(at) => completed_call(frames, at),
        None => Ok(None),
    }
}

fn completed_call(frames: &[Value], at: u64) -> Result<Option<ToolCall>, AxError> {
    let Rebuilt { content, .. } = rebuilt(
        frames
            .iter()
            .filter(|frame| frame.get("index").and_then(Value::as_u64) == Some(at)),
    )?;
    let Some(block) = content.first() else {
        return Ok(None);
    };
    match super::block_from(block, &format!("content[{at}]"))? {
        ContentBlock::ToolUse { id, name, input } => Ok(Some(ToolCall {
            id,
            name,
            args: input,
        })),
        ContentBlock::Text { .. }
        | ContentBlock::Thinking { .. }
        | ContentBlock::RedactedThinking { .. }
        | ContentBlock::ToolResult { .. }
        | ContentBlock::Image(_) => Ok(None),
    }
}

struct Rebuilt {
    content: Vec<Value>,
    usage: Value,
    stop: Option<String>,
}

fn rebuilt<'f>(frames: impl Iterator<Item = &'f Value>) -> Result<Rebuilt, AxError> {
    let mut blocks: std::collections::BTreeMap<u64, Value> = std::collections::BTreeMap::new();
    let mut text: std::collections::BTreeMap<u64, String> = std::collections::BTreeMap::new();
    let mut json: std::collections::BTreeMap<u64, String> = std::collections::BTreeMap::new();
    let mut signature: std::collections::BTreeMap<u64, String> = std::collections::BTreeMap::new();
    let mut usage = json!({});
    let mut stop = None;
    for frame in frames {
        let Some(map) = frame.as_object() else {
            continue;
        };
        let at = map.get("index").and_then(Value::as_u64).unwrap_or_default();
        match map.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                if let Some(held) = map.get("message").and_then(|held| held.get("usage")) {
                    usage = held.clone();
                }
            }
            Some("content_block_start") => {
                if let Some(block) = map.get("content_block") {
                    blocks.insert(at, block.clone());
                }
            }
            Some("content_block_delta") => {
                let Some(delta) = map.get("delta").and_then(Value::as_object) else {
                    continue;
                };
                if let Some(said) = delta.get("text").and_then(Value::as_str) {
                    text.entry(at).or_default().push_str(said);
                }
                if let Some(said) = delta.get("partial_json").and_then(Value::as_str) {
                    json.entry(at).or_default().push_str(said);
                }
                if let Some(said) = delta.get("thinking").and_then(Value::as_str) {
                    text.entry(at).or_default().push_str(said);
                }

                if let Some(said) = delta.get("signature").and_then(Value::as_str) {
                    signature.entry(at).or_default().push_str(said);
                }
            }
            Some("message_delta") => {
                if let Some(held) = map.get("delta").and_then(|held| held.get("stop_reason")) {
                    stop = held.as_str().map(str::to_owned);
                }

                if let Some(held) = map.get("usage").and_then(Value::as_object)
                    && let Some(counted) = usage.as_object_mut()
                {
                    for (name, value) in held {
                        counted.insert(name.clone(), value.clone());
                    }
                }
            }
            Some("error") => {
                let kind = map
                    .get("error")
                    .and_then(|held| held.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("an error without a type");
                return Err(provider_err(
                    "read a streamed answer",
                    &ProviderFailure::Reported { kind },
                ));
            }
            _ => {}
        }
    }
    let mut content = Vec::new();
    for (at, mut block) in blocks {
        if let Some(map) = block.as_object_mut() {
            if let Some(said) = text.get(&at) {
                let field = if map.contains_key("thinking") {
                    "thinking"
                } else {
                    "text"
                };
                map.insert(field.to_owned(), Value::String(said.clone()));
            }
            if let Some(said) = signature.get(&at) {
                map.insert("signature".to_owned(), Value::String(said.clone()));
            }
            if let Some(said) = json.get(&at) {
                let tool = map
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unnamed")
                    .to_owned();
                map.insert("input".to_owned(), settled_tool_arguments(&tool, at, said)?);
            }
        }
        content.push(block);
    }
    Ok(Rebuilt {
        content,
        usage,
        stop,
    })
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
    use super::{completed_call, settled};
    use kernel::DialectKind;
    use serde_json::json;

    fn thinking_stream(signature: Option<&str>) -> Vec<serde_json::Value> {
        let mut frames = vec![
            json!({"type": "message_start", "message": {"usage": {"input_tokens": 4}}}),
            json!({"type": "content_block_start", "index": 0,
                   "content_block": {"type": "thinking", "thinking": "", "signature": ""}}),
            json!({"type": "content_block_delta", "index": 0,
                   "delta": {"type": "thinking_delta", "thinking": "two parts"}}),
        ];
        if let Some(signature) = signature {
            frames.push(json!({"type": "content_block_delta", "index": 0,
                               "delta": {"type": "signature_delta", "signature": signature}}));
        }
        frames.push(json!({"type": "content_block_stop", "index": 0}));
        frames.push(
            json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"},
                           "usage": {"output_tokens": 9}}),
        );
        frames
    }

    #[test]
    fn a_signature_delta_reaches_the_next_turn_byte_for_byte() {
        let settled = settled(&thinking_stream(Some("WaUjzkyp"))).unwrap();
        assert_eq!(settled["content"][0]["signature"], "WaUjzkyp");
        assert_eq!(settled["content"][0]["thinking"], "two parts");

        let canonical = crate::dialect::response_from_wire(DialectKind::Anthropic, &settled)
            .expect("a settled stream is a response");
        assert_eq!(
            crate::dialect::response_wire(DialectKind::Anthropic, &canonical).unwrap()["content"]
                [0]["signature"],
            "WaUjzkyp",
            "a thinking block whose signature changed is one the provider refuses"
        );
    }

    #[test]
    fn a_thinking_block_that_lost_its_signature_is_refused_here() {
        let settled = settled(&thinking_stream(None)).unwrap();
        let refused = crate::dialect::response_from_wire(DialectKind::Anthropic, &settled)
            .expect_err("a thinking block without a signature cannot be sent back");
        assert_eq!(refused.code(), &kernel::AxCode::WireMismatch);
        assert!(!refused.recovery().is_empty());
    }

    #[test]
    fn an_error_frame_mid_stream_is_the_failure_it_reports() {
        let frames = vec![
            json!({"type": "message_start", "message": {"usage": {"input_tokens": 4}}}),
            json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}),
        ];
        let refused = settled(&frames).expect_err("an error frame is not an answer");
        assert_eq!(
            (
                refused.subject(),
                serde_json::to_value(&refused).unwrap()["retry"].clone()
            ),
            ("the stream reported overloaded_error", json!("yes"))
        );
    }

    #[test]
    fn tool_arguments_cut_mid_value_are_refused_not_emptied() {
        let frames = vec![
            json!({"type": "content_block_start", "index": 0,
                   "content_block": {"type": "tool_use", "id": "tu_1", "name": "exec",
                                     "input": {}}}),
            json!({"type": "content_block_delta", "index": 0,
                   "delta": {"type": "input_json_delta", "partial_json": "{\"cmd\": \"rm "}}),
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"},
                   "usage": {"output_tokens": 3}}),
        ];
        let refused = settled(&frames).expect_err("half a tool call is not a tool call");
        assert_eq!(refused.code(), &kernel::AxCode::Provider);
        assert!(
            refused.subject().contains("exec"),
            "the refusal has to name the tool: {}",
            refused.subject()
        );
    }

    #[test]
    fn a_tool_call_is_handed_over_when_its_block_stops() {
        let frames = vec![
            json!({"type": "message_start", "message": {"usage": {"input_tokens": 4}}}),
            json!({"type": "content_block_start", "index": 0,
                   "content_block": {"type": "tool_use", "id": "tu_1", "name": "read",
                                     "input": {}}}),
            json!({"type": "content_block_delta", "index": 0,
                   "delta": {"type": "input_json_delta", "partial_json": "{\"path\": "}}),
            json!({"type": "content_block_start", "index": 1,
                   "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 0,
                   "delta": {"type": "input_json_delta", "partial_json": "\"a.md\"}"}}),
            json!({"type": "content_block_stop", "index": 0}),
        ];
        let early = completed_call(&frames, 0)
            .unwrap()
            .expect("the block at 0 is a tool call");
        assert_eq!(completed_call(&frames, 1).unwrap(), None);

        let mut whole = frames;
        whole.push(json!({"type": "content_block_delta", "index": 1,
                          "delta": {"type": "text_delta", "text": "reading"}}));
        whole.push(
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"},
                          "usage": {"output_tokens": 9}}),
        );
        let canonical =
            crate::dialect::response_from_wire(DialectKind::Anthropic, &settled(&whole).unwrap())
                .unwrap();
        let settled_calls = kernel::ModelReturn::from_response(canonical, None)
            .unwrap()
            .calls;
        assert_eq!(settled_calls, vec![early]);
    }
}
