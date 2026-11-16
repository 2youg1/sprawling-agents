// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::request::{sample_request, sample_seeing};
use super::*;
use kernel::ContentBlock;

#[test]
fn anthropic_sees_a_picture_as_a_base64_source_block() {
    let (chat, images) = sample_seeing();
    let wire = request_wire(
        DialectKind::Anthropic,
        &chat,
        &images,
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    insta::assert_snapshot!(serde_json::to_string_pretty(&wire).unwrap());
}

#[test]
fn openai_sees_a_picture_as_a_data_url_part() {
    let (chat, images) = sample_seeing();
    let wire = request_wire(
        DialectKind::OpenAi,
        &chat,
        &images,
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    insta::assert_snapshot!(serde_json::to_string_pretty(&wire).unwrap());
}

#[test]
fn a_tool_message_cannot_carry_a_picture_so_the_picture_follows_it() {
    // The loss the documented spelling takes, asserted rather than
    // described: the `tool` message stays text, and the attachment
    // lands in the user message immediately after it.
    let (chat, images) = sample_seeing();
    let wire = request_wire(
        DialectKind::OpenAi,
        &chat,
        &images,
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    let messages = wire["messages"].as_array().unwrap();
    let at = messages
        .iter()
        .position(|m| m["role"] == "tool")
        .expect("the tool result crosses as a tool message");
    assert!(
        messages[at]["content"].is_string(),
        "a tool message on this wire is a string, pictures or not"
    );
    let following = &messages[at.saturating_add(1)];
    assert_eq!(following["role"], "user");
    assert_eq!(following["content"][0]["type"], "image_url");
}

#[test]
fn a_host_that_takes_tool_pictures_keeps_them_on_the_tool_message() {
    use crate::provider::preset::ToolPictureCarry;
    let (chat, images) = sample_seeing();
    let wire = request_wire(
        DialectKind::OpenAi,
        &chat,
        &images,
        ChatSpelling {
            chat_tool_pictures: ToolPictureCarry::InToolContent,
            ..ChatSpelling::DOCUMENTED
        },
    )
    .unwrap();
    let messages = wire["messages"].as_array().unwrap();
    let tool = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("the tool result crosses as a tool message");
    let parts = tool["content"].as_array().expect("parts, not a string");
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[1]["type"], "image_url");
    let following_tool_images: Vec<_> = messages
        .iter()
        .filter(|m| m["role"] == "user")
        .flat_map(|m| m["content"].as_array().into_iter().flatten())
        .filter(|part| part["type"] == "image_url")
        .collect();
    // The person's own attached picture still rides a user message.
    assert_eq!(following_tool_images.len(), 1);
}

/// DeepSeek's chat face accepts images on user messages only. A
/// picture produced by `read` therefore follows its text-only tool
/// result instead of turning that result into an invalid parts array.
#[test]
fn deepseek_chat_tool_pictures_follow_in_a_user_message() {
    use crate::provider::preset::chat_spelling;
    let (mut chat, images) = sample_seeing();
    chat.tools.to_mut()[0].name = kernel::ToolName::parse("read").unwrap();
    for block in &mut chat.messages.to_mut()[1].content {
        if let ContentBlock::ToolUse { name, input, .. } = block {
            *name = kernel::ToolName::parse("read").unwrap();
            *input = kernel::Payload::new(
                serde_json::json!({"file_path": "hall/dropped/picture.png"})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        }
    }
    chat.messages.to_mut()[1].content.insert(
        0,
        ContentBlock::Thinking {
            thinking: "the read result needs its picture".to_owned(),
            signature: "a signature never returns on chat".to_owned(),
        },
    );
    let wire = request_wire(
        DialectKind::OpenAi,
        &chat,
        &images,
        chat_spelling("https://api.deepseek.com"),
    )
    .unwrap();
    let messages = wire["messages"].as_array().unwrap();
    let assistant_at = messages
        .iter()
        .position(|message| message["role"] == "assistant")
        .expect("the calling turn crosses as an assistant message");
    let expected = serde_json::json!([
        {
            "role": "assistant",
            "content": "reading",
            "reasoning_content": "the read result needs its picture",
            "tool_calls": [{
                "id": "tu_1",
                "type": "function",
                "function": {
                    "name": "read",
                    "arguments": "{\"file_path\":\"hall/dropped/picture.png\"}"
                }
            }]
        },
        {"role": "tool", "tool_call_id": "tu_1", "content": "ok"},
        {"role": "user", "content": [{
            "type": "image_url",
            "image_url": {"url": "data:image/jpeg;base64,dGhlLXRvb2wtYnl0ZXM="}
        }]}
    ]);
    assert_eq!(
        &messages[assistant_at..],
        expected.as_array().unwrap().as_slice(),
        "thinking, the read result and its picture use DeepSeek's documented chat shape"
    );
}

/// Kimi's documented multimodal tool shape keeps the picture on the
/// tool message; that host rule is independent of DeepSeek's.
#[test]
fn kimi_spellings_keep_tool_pictures_on_the_tool_message() {
    use crate::provider::preset::{ToolPictureCarry, chat_spelling};
    let (chat, images) = sample_seeing();
    for base in [
        "https://api.moonshot.ai/v1",
        "https://api.moonshot.cn/v1",
        "https://api.kimi.com/coding/v1",
    ] {
        let spelling = chat_spelling(base);
        assert_eq!(
            spelling.chat_tool_pictures,
            ToolPictureCarry::InToolContent,
            "{base}"
        );
        let wire = request_wire(DialectKind::OpenAi, &chat, &images, spelling).unwrap();
        let messages = wire["messages"].as_array().unwrap();
        let tool_at = messages
            .iter()
            .position(|message| message["role"] == "tool")
            .unwrap_or_else(|| panic!("{base}: expected a tool message"));
        let parts = messages[tool_at]["content"]
            .as_array()
            .unwrap_or_else(|| panic!("{base}: tool content must be parts, not a string"));
        assert_eq!(parts[1]["type"], "image_url", "{base}");
        assert!(messages.get(tool_at.saturating_add(1)).is_none(), "{base}");
    }
}

#[test]
fn a_picture_whose_bytes_nobody_resolved_is_refused() {
    let (chat, _) = sample_seeing();
    let err = request_wire(
        DialectKind::Anthropic,
        &chat,
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap_err();
    assert_eq!(*err.code(), kernel::AxCode::WireMismatch);
}

#[test]
fn anthropic_request_wire_is_pinned() {
    let wire = request_wire(
        DialectKind::Anthropic,
        &sample_request(),
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    insta::assert_snapshot!(serde_json::to_string_pretty(&wire).unwrap());
}

#[test]
fn openai_request_wire_is_pinned() {
    let wire = request_wire(
        DialectKind::OpenAi,
        &sample_request(),
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    insta::assert_snapshot!(serde_json::to_string_pretty(&wire).unwrap());
}

#[test]
fn anthropic_breakpoints_land_on_marked_blocks_only() {
    let wire = request_wire(
        DialectKind::Anthropic,
        &sample_request(),
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    let system = wire["system"].as_array().unwrap();
    assert!(
        system
            .iter()
            .all(|b| b["cache_control"]["type"] == "ephemeral")
    );
    let mut unmarked = sample_request();
    unmarked.system[1].cache = false;
    let wire = request_wire(
        DialectKind::Anthropic,
        &unmarked,
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    assert!(wire["system"][1].get("cache_control").is_none());
}

#[test]
fn anthropic_marked_message_carries_its_breakpoint_on_the_last_block() {
    let mut req = sample_request();
    let tail = req.messages.len() - 1;
    req.breakpoint = kernel::MessageBreakpoint::Tail;
    let wire = request_wire(
        DialectKind::Anthropic,
        &req,
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    let messages = wire["messages"].as_array().unwrap();
    let blocks = messages[tail]["content"].as_array().unwrap();
    assert_eq!(
        blocks.last().unwrap()["cache_control"],
        serde_json::json!({ "type": "ephemeral" })
    );
    let marked: usize = messages
        .iter()
        .flat_map(|m| m["content"].as_array().unwrap())
        .filter(|b| b.get("cache_control").is_some())
        .count();
    assert_eq!(marked, 1);
}

#[test]
fn tool_shapes_survive_both_request_dialects() {
    let req = sample_request();
    let anthropic = request_wire(
        DialectKind::Anthropic,
        &req,
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    assert_eq!(anthropic["tools"][0]["name"], "exec");
    assert_eq!(anthropic["tools"][0]["input_schema"]["type"], "object");
    let openai = request_wire(
        DialectKind::OpenAi,
        &req,
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    assert_eq!(openai["tools"][0]["function"]["name"], "exec");
    assert_eq!(
        openai["tools"][0]["function"]["parameters"]["type"],
        "object"
    );
    // The assistant tool_use crosses as a tool_call with the same id.
    let calls = openai["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|m| m.get("tool_calls"))
        .unwrap();
    assert_eq!(calls[0]["id"], "tu_1");
}

#[test]
fn openai_drops_thinking_because_it_cannot_spell_it() {
    // Chat Completions has no counterpart and demands no round trip.
    // The canonical record keeps the block; only the wire loses it,
    // and the dialect is a pure function, so replay can still derive
    // the bytes that were actually sent.
    let mut req = sample_request();
    req.messages.to_mut()[1].content.insert(
        0,
        ContentBlock::Thinking {
            thinking: "two parts".to_owned(),
            signature: "WaUjzkyp".to_owned(),
        },
    );
    let out = request_wire(
        DialectKind::OpenAi,
        &req,
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    assert!(
        !out.to_string().contains("WaUjzkyp"),
        "a signature the other provider cannot verify does not belong on its wire"
    );
    assert!(out.to_string().contains("reading"));
}

fn chat_spelled(spelling: ChatSpelling, req: &ChatRequest) -> Value {
    request_wire(DialectKind::OpenAi, req, &ImageBytes::default(), spelling).unwrap()
}

/// OpenAI's reasoning models answer 400 to `max_tokens`, and a
/// server that reads only `max_tokens` drops a ceiling written
/// under the other name; the host decides which name is read.
#[test]
fn the_chat_face_writes_the_ceiling_under_the_name_the_host_reads() {
    use crate::provider::preset::CeilingField;
    let req = sample_request();
    let documented = chat_spelled(ChatSpelling::DOCUMENTED, &req);
    assert_eq!(documented["max_tokens"], 4096);
    assert!(documented.get("max_completion_tokens").is_none());
    let replaced = chat_spelled(
        ChatSpelling {
            ceiling: CeilingField::MaxCompletionTokens,
            ..ChatSpelling::DOCUMENTED
        },
        &req,
    );
    assert_eq!(replaced["max_completion_tokens"], 4096);
    assert!(replaced.get("max_tokens").is_none(), "{replaced}");
}

/// DeepSeek's thinking mode answers 400 to a request with tools
/// whose history lacks the earlier `reasoning_content`, and every
/// dispatch in this city carries tools.
#[test]
fn a_host_that_reads_reasoning_back_gets_the_text_and_never_the_signature() {
    use crate::provider::preset::ReasoningReturn;
    let mut req = sample_request();
    req.messages.to_mut()[1].content.insert(
        0,
        ContentBlock::Thinking {
            thinking: "two parts".to_owned(),
            signature: "WaUjzkyp".to_owned(),
        },
    );
    let returned = chat_spelled(
        ChatSpelling {
            reasoning: ReasoningReturn::AsReasoningContent,
            ..ChatSpelling::DOCUMENTED
        },
        &req,
    );
    let assistant = returned["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "assistant")
        .unwrap();
    assert_eq!(assistant["reasoning_content"], "two parts");
    assert!(!returned.to_string().contains("WaUjzkyp"));
    let dropped = chat_spelled(ChatSpelling::DOCUMENTED, &req);
    assert!(!dropped.to_string().contains("reasoning_content"));
}
