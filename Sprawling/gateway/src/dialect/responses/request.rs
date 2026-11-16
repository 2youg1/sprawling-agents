// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, ChatRequest, ContentBlock, ImageRef, Role};
use serde_json::{Map, Value, json};

use crate::dialect::ImageBytes;
use crate::mismatch::mismatch;
use crate::provider::preset::{ChatSpelling, ToolPictureCarry};

fn image_part(picture: &ImageRef, images: &ImageBytes) -> Result<Value, AxError> {
    let data = images.encoded(&picture.locator)?;
    Ok(json!({
        "type": "input_image",
        "image_url": format!("data:{};base64,{data}", picture.media_type.mime()),
        "detail": "auto",
    }))
}

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

fn function_output(
    output: &str,
    attachments: &[ImageRef],
    images: &ImageBytes,
    carry: ToolPictureCarry,
) -> Result<Value, AxError> {
    if attachments.is_empty() || carry == ToolPictureCarry::FollowingUser {
        return Ok(Value::String(output.to_owned()));
    }
    let mut parts = Vec::new();
    if !output.is_empty() {
        parts.push(json!({ "type": "input_text", "text": output }));
    }
    for picture in attachments {
        parts.push(image_part(picture, images)?);
    }
    Ok(Value::Array(parts))
}

fn user_items(
    content: &[ContentBlock],
    images: &ImageBytes,
    carry: ToolPictureCarry,
    into: &mut Vec<Value>,
) -> Result<(), AxError> {
    for block in content {
        if let ContentBlock::ToolResult {
            tool_use_id,
            content: output,
            attachments,
            ..
        } = block
        {
            into.push(json!({
                "type": "function_call_output",
                "call_id": tool_use_id,
                "output": function_output(output, attachments, images, carry)?,
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
    if !parts.is_empty() {
        into.push(json!({ "role": "user", "content": parts }));
    }
    Ok(())
}

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

pub(crate) fn request(
    req: &ChatRequest,
    images: &ImageBytes,
    spelling: ChatSpelling,
) -> Result<Value, AxError> {
    let mut input = Vec::new();
    if let Some(system) = system_item(req) {
        input.push(system);
    }
    for message in req.messages.iter() {
        match message.role {
            Role::User => user_items(
                &message.content,
                images,
                spelling.responses_tool_pictures,
                &mut input,
            )?,
            Role::Assistant => assistant_items(&message.content, &mut input)?,
        }
    }
    let mut root = Map::new();
    root.insert("model".to_owned(), Value::String(req.model.clone()));
    root.insert("input".to_owned(), Value::Array(input));

    if let Some(ceiling) = req.max_tokens {
        root.insert(
            "max_output_tokens".to_owned(),
            Value::Number(ceiling.get().into()),
        );
    }

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
    use crate::provider::preset::ChatSpelling;

    #[test]
    fn a_cached_segment_carries_the_breakpoint_this_face_reads() {
        let wire = request(
            &sample_request(),
            &ImageBytes::default(),
            ChatSpelling::DOCUMENTED,
        )
        .unwrap();
        let parts = wire["input"][0]["content"].as_array().unwrap();
        assert_eq!(wire["input"][0]["role"], "developer");
        assert_eq!(parts.len(), 2, "one part per system segment");
        for part in parts {
            assert_eq!(part["type"], "input_text");
            assert_eq!(part["prompt_cache_breakpoint"]["mode"], "explicit");
        }
        let mut unmarked = sample_request();
        unmarked.system[1].cache = false;
        let wire = request(&unmarked, &ImageBytes::default(), ChatSpelling::DOCUMENTED).unwrap();
        assert!(
            wire["input"][0]["content"][1]
                .get("prompt_cache_breakpoint")
                .is_none()
        );
    }

    #[test]
    fn a_tool_result_is_its_own_item_and_precedes_the_turn_it_sits_in() {
        let (chat, images) = sample_seeing();
        let wire = request(&chat, &images, ChatSpelling::DOCUMENTED).unwrap();
        let input = wire["input"].as_array().unwrap();
        let output_at = input
            .iter()
            .position(|item| item["type"] == "function_call_output")
            .expect("the tool result crosses as an item of its own");
        assert_eq!(input[output_at]["call_id"], "tu_1");
        assert!(input[output_at]["output"].is_string());

        let following = &input[output_at.saturating_add(1)];
        assert_eq!(following["role"], "user");
        assert_eq!(following["content"][0]["type"], "input_image");
    }

    #[test]
    fn deepseek_responses_tool_pictures_stay_on_the_function_output() {
        let (chat, images) = sample_seeing();
        let wire = request(
            &chat,
            &images,
            crate::provider::preset::chat_spelling("https://api.deepseek.com"),
        )
        .unwrap();
        let input = wire["input"].as_array().unwrap();
        let output = input
            .iter()
            .find(|item| item["type"] == "function_call_output")
            .expect("the tool result crosses as an item of its own");
        let parts = output["output"].as_array().expect("parts, not a string");
        assert_eq!(parts[0]["type"], "input_text");
        assert_eq!(parts[1]["type"], "input_image");
        let following_images: Vec<_> = input
            .iter()
            .filter(|item| item["role"] == "user")
            .flat_map(|item| item["content"].as_array().into_iter().flatten())
            .filter(|part| part["type"] == "input_image")
            .collect();

        assert_eq!(following_images.len(), 1);
    }

    #[test]
    fn a_call_and_the_result_that_answers_it_share_one_identifier() {
        let wire = request(
            &sample_request(),
            &ImageBytes::default(),
            ChatSpelling::DOCUMENTED,
        )
        .unwrap();
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

    #[test]
    fn nothing_this_city_sends_is_stored_upstream() {
        let wire = request(
            &sample_request(),
            &ImageBytes::default(),
            ChatSpelling::DOCUMENTED,
        )
        .unwrap();
        assert_eq!(wire["store"], false);
    }
}
