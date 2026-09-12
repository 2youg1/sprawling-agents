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

use kernel::AxError;
use kernel::Increment;
use serde_json::{Value, json};

use crate::mismatch::stream_cut;

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
            if let Some(said) = json.get(&at) {
                let parsed = serde_json::from_str::<Value>(said).unwrap_or_else(|_| json!({}));
                map.insert("input".to_owned(), parsed);
            }
        }
        content.push(block);
    }
    Ok(json!({ "content": content, "stop_reason": stop, "usage": usage }))
}
