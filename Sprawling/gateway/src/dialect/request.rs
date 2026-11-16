// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, ChatRequest, DialectKind};
use serde_json::Value;

use super::images::ImageBytes;
use super::responses;
use crate::provider::preset::ChatSpelling;
use crate::{anthropic, openai};

pub fn request_wire(
    kind: DialectKind,
    req: &ChatRequest,
    images: &ImageBytes,
    spelling: ChatSpelling,
) -> Result<Value, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::request(req, images),
        DialectKind::OpenAi => openai::request(req, images, spelling),
        DialectKind::OpenAiResponses => responses::request(req, images, spelling),
    }
}
#[cfg(test)]
use kernel::{ChatMessage, ContentBlock, Payload, Role, SystemBlock, ToolDef, ToolName};
#[cfg(test)]
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
    images
        .insert(&seen, b"the-attached-bytes".to_vec(), None)
        .unwrap();
    images
        .insert(&made, b"the-tool-bytes".to_vec(), None)
        .unwrap();
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
        shared: kernel::SharedPrefix::Unshared,
        effort: None,
        generation: kernel::GenerationSettings::default(),
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
mod tests;
