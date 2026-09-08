// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one OpenAI stream settles into.
//!
//! The provider sends one `choices[0].delta` per chunk and the finish
//! reason on the last one. Tool calls arrive by index with their
//! arguments split across chunks, which is why they are joined before
//! anything reads them.

use kernel::AxError;
use serde_json::{Value, json};

use crate::mismatch::stream_cut;

/// The text one chunk carries, if it carries prose. Absent on the frames
/// that carry a tool call or a finish reason.
pub(crate) fn increment_of(map: &serde_json::Map<String, Value>) -> Option<String> {
    map.get("choices")?
        .as_array()?
        .first()?
        .as_object()?
        .get("delta")?
        .as_object()?
        .get("content")?
        .as_str()
        .map(str::to_owned)
}

/// OpenAI streams one `choices[0].delta` per chunk and the finish reason
/// on the last one. Tool calls arrive by index with their arguments split
/// across chunks, which is why they are joined before being read.
pub(crate) fn settled(frames: &[Value]) -> Result<Value, AxError> {
    let mut said = String::new();
    let mut calls: std::collections::BTreeMap<u64, (String, String, String)> =
        std::collections::BTreeMap::new();
    let mut finish = None;
    let mut usage = None;
    for frame in frames {
        let Some(map) = frame.as_object() else {
            continue;
        };
        if let Some(held) = map.get("usage")
            && !held.is_null()
        {
            usage = Some(held.clone());
        }
        let Some(first) = map
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(Value::as_object)
        else {
            continue;
        };
        if let Some(held) = first.get("finish_reason").and_then(Value::as_str) {
            finish = Some(held.to_owned());
        }
        let Some(delta) = first.get("delta").and_then(Value::as_object) else {
            continue;
        };
        if let Some(text) = delta.get("content").and_then(Value::as_str) {
            said.push_str(text);
        }
        for call in delta
            .get("tool_calls")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            let Some(one) = call.as_object() else {
                continue;
            };
            let at = one.get("index").and_then(Value::as_u64).unwrap_or_default();
            let held = calls.entry(at).or_default();
            if let Some(id) = one.get("id").and_then(Value::as_str) {
                held.0 = id.to_owned();
            }
            let Some(function) = one.get("function").and_then(Value::as_object) else {
                continue;
            };
            if let Some(name) = function.get("name").and_then(Value::as_str) {
                held.1 = name.to_owned();
            }
            if let Some(part) = function.get("arguments").and_then(Value::as_str) {
                held.2.push_str(part);
            }
        }
    }
    let Some(finish) = finish else {
        return Err(stream_cut(
            "the stream ended without the chunk that says why the model stopped",
        ));
    };
    let mut message = json!({ "role": "assistant", "content": said });
    if !calls.is_empty()
        && let Some(map) = message.as_object_mut()
    {
        let wired: Vec<Value> = calls
            .into_values()
            .map(|(id, name, arguments)| {
                json!({
                    "id": id,
                    "type": "function",
                    "function": { "name": name, "arguments": arguments },
                })
            })
            .collect();
        map.insert("tool_calls".to_owned(), Value::Array(wired));
    }
    Ok(json!({
        "choices": [{ "message": message, "finish_reason": finish }],
        "usage": usage.unwrap_or_else(|| json!({})),
    }))
}
