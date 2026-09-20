// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one Anthropic stream settles into.
//!
//! The provider sends one `content_block_start` per block, then deltas
//! against it by index, then a `message_delta` carrying the stop reason
//! and the output count. Rebuilding the blocks in index order is what
//! keeps a tool call that arrived interleaved with prose where it was.
//!
//! A thinking block arrives in two streams: its text in
//! `thinking_delta` and the provider's `signature_delta` at the end.
//! The signature is what the provider verifies the reasoning against on
//! the next turn, so a block rebuilt without it is refused rather than
//! sent back: <https://platform.claude.com/docs/en/build-with-claude/thinking>.

use kernel::AxError;
use kernel::Increment;
use serde_json::{Value, json};

use crate::mismatch::{settled_tool_arguments, stream_cut};

/// What one `content_block_delta` carries, if it carries either stream.
///
/// Reading the delta's own type rather than the presence of a field is
/// what keeps a tool's arguments out of a person's reading pane: a
/// partial `input_json_delta` is not a shorter tool argument.
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

/// Anthropic streams one `content_block_start` per block, then deltas
/// against it by index, then `message_delta` with the stop reason and the
/// output count. The blocks are rebuilt in index order so a tool call
/// that arrived interleaved with text still lands where it was.
pub(crate) fn settled(frames: &[Value]) -> Result<Value, AxError> {
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
                // Sent in parts like every other delta, and joined the
                // same way: the provider verifies the whole string.
                if let Some(said) = delta.get("signature").and_then(Value::as_str) {
                    signature.entry(at).or_default().push_str(said);
                }
            }
            Some("message_delta") => {
                if let Some(held) = map.get("delta").and_then(|held| held.get("stop_reason")) {
                    stop = held.as_str().map(str::to_owned);
                }
                // The output count arrives here rather than at the start,
                // because it is not known until the model stops.
                if let Some(held) = map.get("usage").and_then(Value::as_object)
                    && let Some(counted) = usage.as_object_mut()
                {
                    for (name, value) in held {
                        counted.insert(name.clone(), value.clone());
                    }
                }
            }
            _ => {}
        }
    }
    let Some(stop) = stop else {
        return Err(stream_cut(
            "the stream ended without the frame that says why the model stopped",
        ));
    };
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
    Ok(json!({ "content": content, "stop_reason": stop, "usage": usage }))
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
    use super::settled;
    use kernel::DialectKind;
    use serde_json::json;

    /// The frames one thinking block arrives in: the block first, then
    /// its text, then the signature the provider verifies next turn.
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

    /// The provider verifies the signature against the reasoning it
    /// issued, so a stream that dropped it earns a 400 on the next turn
    /// of the same conversation rather than here.
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

    /// An empty signature is not a thinking block with less in it: it
    /// is one this build must not send back. Refusing here names the
    /// stream that lost it, instead of leaving a 400 next turn.
    #[test]
    fn a_thinking_block_that_lost_its_signature_is_refused_here() {
        let settled = settled(&thinking_stream(None)).unwrap();
        let refused = crate::dialect::response_from_wire(DialectKind::Anthropic, &settled)
            .expect_err("a thinking block without a signature cannot be sent back");
        assert_eq!(refused.code(), &kernel::AxCode::WireMismatch);
        assert!(!refused.recovery().is_empty());
    }

    /// Half a tool call is not a tool call with fewer arguments: an
    /// `exec {}` assembled from a cut stream passes the gates and runs.
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
}
