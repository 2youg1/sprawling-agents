// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One canonical `ChatRequest` onto the responses face.

use kernel::{AxError, ChatRequest, ContentBlock, Effort, ImageRef, Role};
use serde_json::{Map, Value, json};

use crate::dialect::ImageBytes;
use crate::mismatch::mismatch;

/// The effort word this face takes. The provider's `ReasoningEffort`
/// accepts `none`, `minimal`, `low`, `medium`, `high`, `xhigh` and
/// `max`; this city's ladder is a subset of those, so every level is
/// spelled rather than approximated.
fn effort_field(effort: Effort) -> &'static str {
    match effort {
        Effort::None => "none",
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
        Effort::XHigh => "xhigh",
        Effort::Max => "max",
    }
}

/// One picture as this face spells it: a data URL in an `input_image`
/// part. `detail` is required by the schema, and `auto` is the value
/// that leaves the choice to the provider — which is the honest
/// statement, because nothing in this city has an opinion about it.
fn image_part(picture: &ImageRef, images: &ImageBytes) -> Result<Value, AxError> {
    let data = images.encoded(&picture.locator)?;
    Ok(json!({
        "type": "input_image",
        "image_url": format!("data:{};base64,{data}", picture.media_type.mime()),
        "detail": "auto",
    }))
}

/// The system segments, as the first input item.
///
/// A `developer` message rather than the `instructions` field, because
/// `instructions` is one string and this city has four segments with
/// cache breakpoints on their edges. Each segment becomes its own
/// `input_text` part, and a segment marked `cache` carries the
/// breakpoint the provider reads.
fn system_item(req: &ChatRequest) -> Option<Value> {
    if req.system.is_empty() {
        return None;
    }
    let parts: Vec<Value> = req
        .system
        .iter()
        .map(|block| {
            let mut part = Map::new();
            part.insert("type".to_owned(), Value::String("input_text".to_owned()));
            part.insert("text".to_owned(), Value::String(block.text.clone()));
            if block.cache {
                part.insert(
                    "prompt_cache_breakpoint".to_owned(),
                    json!({ "mode": "explicit" }),
                );
            }
            Value::Object(part)
        })
        .collect();
    Some(json!({ "role": "developer", "content": parts }))
}

/// One user turn: every tool result it closes, then whatever the person
/// said and showed.
///
/// The results come first because each one answers a call the previous
/// assistant turn made, and this face reads the array in order.
fn user_items(
    content: &[ContentBlock],
    images: &ImageBytes,
    into: &mut Vec<Value>,
) -> Result<(), AxError> {
    for block in content {
        if let ContentBlock::ToolResult {
            tool_use_id,
            content: output,
            ..
        } = block
        {
            into.push(json!({
                "type": "function_call_output",
                "call_id": tool_use_id,
                "output": output,
            }));
        }
    }
    let mut parts = Vec::new();
    for block in content {
        match block {
            ContentBlock::Text { text } if !text.is_empty() => {
                parts.push(json!({ "type": "input_text", "text": text }));
            }
            ContentBlock::Image(picture) => parts.push(image_part(picture, images)?),
            ContentBlock::ToolResult { attachments, .. } => {
                // A picture a tool produced rides in the same turn as
                // the output it belongs to. This face has no image part
                // inside `function_call_output`, so what is lost is the
                // statement that the picture came out of that call, and
                // the picture itself arrives.
                for picture in attachments {
                    parts.push(image_part(picture, images)?);
                }
            }
            ContentBlock::Text { .. }
            | ContentBlock::Thinking { .. }
            | ContentBlock::RedactedThinking { .. }
            | ContentBlock::ToolUse { .. } => {}
        }
    }
    if !parts.is_empty() {
        into.push(json!({ "role": "user", "content": parts }));
    }
    Ok(())
}

/// One assistant turn: what it said, then every call it made.
///
/// A `Thinking` block is not written. This face accepts a `reasoning`
/// item only with the identifier and the encrypted content the provider
/// issued, neither of which this city stores, and a reconstructed one
/// is refused at the call.
fn assistant_items(content: &[ContentBlock], into: &mut Vec<Value>) -> Result<(), AxError> {
    let mut said = String::new();
    for block in content {
        if let ContentBlock::Text { text } = block {
            if !said.is_empty() {
                said.push_str("\n\n");
            }
            said.push_str(text);
        }
    }
    if !said.is_empty() {
        into.push(json!({ "role": "assistant", "content": said }));
    }
    for block in content {
        if let ContentBlock::ToolUse { id, name, input } = block {
            let arguments = serde_json::to_string(input)
                .map_err(|err| mismatch("tool_use.input", &err.to_string()))?;
            into.push(json!({
                "type": "function_call",
                "call_id": id,
                "name": name.as_str(),
                "arguments": arguments,
            }));
        }
    }
    Ok(())
}

pub(crate) fn request(req: &ChatRequest, images: &ImageBytes) -> Result<Value, AxError> {
    let mut input = Vec::new();
    if let Some(system) = system_item(req) {
        input.push(system);
    }
    for message in &req.messages {
        match message.role {
            Role::User => user_items(&message.content, images, &mut input)?,
            Role::Assistant => assistant_items(&message.content, &mut input)?,
        }
    }
    let mut root = Map::new();
    root.insert("model".to_owned(), Value::String(req.model.clone()));
    root.insert("input".to_owned(), Value::Array(input));
    // Absent when no catalogue row states this model's ceiling. The
    // field may be missing on this face, and the provider's own default
    // is a real number where a zero is a reply with nothing in it.
    if let Some(ceiling) = req.max_tokens {
        root.insert(
            "max_output_tokens".to_owned(),
            Value::Number(ceiling.get().into()),
        );
    }
    if let Some(effort) = req.effort {
        root.insert(
            "reasoning".to_owned(),
            json!({ "effort": effort_field(effort) }),
        );
    }
    // Nothing this city sends is retrievable later: it keeps its own
    // history, and a copy held by the provider is a second one that
    // outlives the city's decision to forget.
    root.insert("store".to_owned(), Value::Bool(false));
    if !req.tools.is_empty() {
        let tools: Result<Vec<Value>, AxError> = req
            .tools
            .iter()
            .map(|tool| {
                Ok(json!({
                    "type": "function",
                    "name": tool.name.as_str(),
                    "description": tool.description,
                    "parameters": serde_json::to_value(&tool.input_schema)
                        .map_err(|err| mismatch("tools.parameters", &err.to_string()))?,
                }))
            })
            .collect();
        root.insert("tools".to_owned(), Value::Array(tools?));
    }
    Ok(Value::Object(root))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::dialect::request::{sample_request, sample_seeing};

    /// What this face carries that the chat face cannot: the segment
    /// edges this city already computes, stated to the provider rather
    /// than left to prefix matching.
    #[test]
    fn a_cached_segment_carries_the_breakpoint_this_face_reads() {
        let wire = request(&sample_request(), &ImageBytes::default()).unwrap();
        let parts = wire["input"][0]["content"].as_array().unwrap();
        assert_eq!(wire["input"][0]["role"], "developer");
        assert_eq!(parts.len(), 2, "one part per system segment");
        for part in parts {
            assert_eq!(part["type"], "input_text");
            assert_eq!(part["prompt_cache_breakpoint"]["mode"], "explicit");
        }
        let mut unmarked = sample_request();
        unmarked.system[1].cache = false;
        let wire = request(&unmarked, &ImageBytes::default()).unwrap();
        assert!(
            wire["input"][0]["content"][1]
                .get("prompt_cache_breakpoint")
                .is_none()
        );
    }

    /// A tool result is an item of its own on this face, and it comes
    /// before whatever the person said in the same turn: it answers the
    /// call the previous turn made, and this face reads the array in
    /// order.
    #[test]
    fn a_tool_result_is_its_own_item_and_precedes_the_turn_it_sits_in() {
        let (chat, images) = sample_seeing();
        let wire = request(&chat, &images).unwrap();
        let input = wire["input"].as_array().unwrap();
        let output_at = input
            .iter()
            .position(|item| item["type"] == "function_call_output")
            .expect("the tool result crosses as an item of its own");
        assert_eq!(input[output_at]["call_id"], "tu_1");
        // The picture the tool produced rides in the turn that follows.
        let following = &input[output_at.saturating_add(1)];
        assert_eq!(following["role"], "user");
        assert_eq!(following["content"][0]["type"], "input_image");
    }

    /// A call the model made crosses as a `function_call` item whose
    /// `call_id` is the id the result will answer with.
    #[test]
    fn a_call_and_the_result_that_answers_it_share_one_identifier() {
        let wire = request(&sample_request(), &ImageBytes::default()).unwrap();
        let input = wire["input"].as_array().unwrap();
        let made = input
            .iter()
            .find(|item| item["type"] == "function_call")
            .unwrap();
        assert_eq!(made["name"], "exec");
        assert_eq!(made["call_id"], "tu_1");
        let answered = input
            .iter()
            .find(|item| item["type"] == "function_call_output")
            .unwrap();
        assert_eq!(answered["call_id"], made["call_id"]);
    }

    /// Every level of the ladder reaches this wire under the one name
    /// the provider's `ReasoningEffort` accepts.
    #[test]
    fn every_level_of_the_ladder_reaches_this_wire() {
        let mut req = sample_request();
        assert!(
            request(&req, &ImageBytes::default())
                .unwrap()
                .get("reasoning")
                .is_none(),
            "an unstated effort writes no field: the provider's default is its own business"
        );
        for (level, spelling) in [
            (Effort::None, "none"),
            (Effort::Low, "low"),
            (Effort::Medium, "medium"),
            (Effort::High, "high"),
            (Effort::XHigh, "xhigh"),
            (Effort::Max, "max"),
        ] {
            req.effort = Some(level);
            let wire = request(&req, &ImageBytes::default()).unwrap();
            assert_eq!(wire["reasoning"]["effort"], spelling);
        }
    }

    /// Nothing this city sends is left with the provider to retrieve
    /// later: the city keeps its own history, and a copy held upstream
    /// is a second one that outlives the city's decision to forget.
    #[test]
    fn nothing_this_city_sends_is_stored_upstream() {
        let wire = request(&sample_request(), &ImageBytes::default()).unwrap();
        assert_eq!(wire["store"], false);
    }
}
