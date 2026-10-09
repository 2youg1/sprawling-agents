// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which dialect answers a question, and nothing about how it answers.
//!
//! Five entrances, one `match` each, and a closed set of two: a third
//! dialect is a compile error at all five entrances, which is how it is
//! kept from being approximated with the nearer of the two we already
//! write. What each dialect does with a request
//! is `gateway::anthropic`'s and `gateway::openai`'s; what they share is
//! `gateway::mismatch`'s.
//!
//! The canonical conversation is Anthropic-shaped, so the losses are all
//! on the other path and each is documented where it is taken.
//!
//! No I/O, no state, no clock: byte-for-byte explainable requests are
//! the whole point of writing the wire format ourselves.

//! Request dialects: one ChatRequest, each wire.

use kernel::{AxError, ChatRequest, DialectKind};
use serde_json::Value;

use super::images::ImageBytes;
use super::responses;
use crate::provider::preset::ChatSpelling;
use crate::{anthropic, openai};

/// One canonical request as `kind`'s wire spells it.
///
/// `spelling` is read by the chat face alone: the other two faces each
/// have one spelling, and the chat face's three vendor-decided fields
/// are the host's to state (`provider::preset::chat_spelling`).
pub fn request_wire(
    kind: DialectKind,
    req: &ChatRequest,
    images: &ImageBytes,
    spelling: ChatSpelling,
) -> Result<Value, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::request(req, images),
        DialectKind::OpenAi => openai::request(req, images, spelling),
        DialectKind::OpenAiResponses => responses::request(req, images),
    }
}
#[cfg(test)]
use kernel::{ChatMessage, ContentBlock, Payload, Role, SystemBlock, ToolDef, ToolName};
#[cfg(test)]
/// One conversation that carries a picture two ways: as a block a person
/// attached, and as a picture a tool produced.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helper"
)]
pub(crate) fn sample_seeing() -> (ChatRequest<'static>, ImageBytes) {
    let seen = kernel::ImageRef {
        locator: kernel::Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap(),
        media_type: kernel::ImageType::Png,
        width: 8,
        height: 8,
    };
    let made = kernel::ImageRef {
        locator: kernel::Locator::parse(&format!("cas:b3-{}", "cd".repeat(32))).unwrap(),
        media_type: kernel::ImageType::Jpeg,
        width: 4,
        height: 4,
    };
    let mut chat = sample_request();
    chat.messages.to_mut()[0]
        .content
        .push(ContentBlock::Image(seen.clone()));
    chat.messages.to_mut()[2].content = vec![ContentBlock::ToolResult {
        tool_use_id: "tu_1".to_owned(),
        content: "ok".to_owned(),
        is_error: false,
        attachments: vec![made.clone()],
    }];
    let mut images = ImageBytes::default();
    images.insert(&seen.locator, b"the-attached-bytes".to_vec());
    images.insert(&made.locator, b"the-tool-bytes".to_vec());
    (chat, images)
}
#[cfg(test)]
use serde_json::Map;
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn sample_request() -> ChatRequest<'static> {
    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::String("object".to_owned()));
    let mut args = Map::new();
    args.insert("path".to_owned(), Value::String("notes.md".to_owned()));
    ChatRequest {
        model: "sonnet".to_owned(),
        max_tokens: kernel::Ceiling::new(4096),
        system: vec![
            SystemBlock {
                text: "city".to_owned(),
                cache: true,
            },
            SystemBlock {
                text: "building".to_owned(),
                cache: true,
            },
        ],
        messages: vec![
            ChatMessage {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Task: probe".to_owned(),
                }],
            },
            ChatMessage {
                role: Role::Assistant,
                content: vec![
                    ContentBlock::Text {
                        text: "reading".to_owned(),
                    },
                    ContentBlock::ToolUse {
                        id: "tu_1".to_owned(),
                        name: ToolName::parse("exec").unwrap(),
                        input: Payload::new(args).unwrap(),
                    },
                ],
            },
            ChatMessage {
                role: Role::User,
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: "tu_1".to_owned(),
                    content: "ok".to_owned(),
                    is_error: false,
                    attachments: Vec::new(),
                }],
            },
        ]
        .into(),
        tools: vec![ToolDef {
            name: ToolName::parse("exec").unwrap(),
            description: "run things".to_owned(),
            input_schema: Payload::new(schema).unwrap(),
        }]
        .into(),
        breakpoint: kernel::MessageBreakpoint::Unmarked,
        effort: None,
    }
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
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
        // The loss this dialect takes, asserted rather than described:
        // the `tool` message stays text, and the attachment lands in the
        // user message immediately after it.
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
}
