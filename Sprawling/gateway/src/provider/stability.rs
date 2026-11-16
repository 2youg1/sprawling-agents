// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test module"
)]

use kernel::{ChatMessage, ChatRequest, ContentBlock, DialectKind, Role, SystemBlock};

use crate::dialect::{ImageBytes, request_wire};
use crate::provider::preset::ChatSpelling;
use crate::provider::registry::ConnectionKind;

const EVERY_CONNECTION: [ConnectionKind; 3] = [
    ConnectionKind::OpenAiCompat,
    ConnectionKind::Responses,
    ConnectionKind::AnthropicNative,
];

fn dispatch() -> ChatRequest<'static> {
    ChatRequest {
        model: "a-model".to_owned(),
        max_tokens: kernel::Ceiling::new(4096),
        system: vec![
            SystemBlock {
                text: "city norms".to_owned(),
                cache: true,
            },
            SystemBlock {
                text: "building rules".to_owned(),
                cache: true,
            },
        ],
        messages: vec![ChatMessage {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Task: probe".to_owned(),
            }],
        }]
        .into(),
        tools: std::borrow::Cow::Borrowed(&[]),
        breakpoint: kernel::MessageBreakpoint::Unmarked,
        shared: kernel::SharedPrefix::Unshared,
        effort: None,
        generation: kernel::GenerationSettings::default(),
    }
}

fn system_on_the_wire(kind: ConnectionKind, chat: &ChatRequest) -> String {
    let wire = request_wire(
        kind.wire(),
        chat,
        &ImageBytes::default(),
        ChatSpelling::DOCUMENTED,
    )
    .unwrap();
    match kind.wire() {
        DialectKind::Anthropic => wire["system"]
            .as_array()
            .unwrap()
            .iter()
            .map(|block| block["text"].as_str().unwrap().to_owned())
            .collect::<Vec<String>>()
            .join("\n\n"),
        DialectKind::OpenAi => wire["messages"][0]["content"].as_str().unwrap().to_owned(),

        DialectKind::OpenAiResponses => wire["input"][0]["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| part["text"].as_str().unwrap().to_owned())
            .collect::<Vec<String>>()
            .join("\n\n"),
    }
}

#[test]
fn two_dispatches_of_one_configuration_send_the_same_bytes() {
    for kind in EVERY_CONNECTION {
        let first = request_wire(
            kind.wire(),
            &dispatch(),
            &ImageBytes::default(),
            ChatSpelling::DOCUMENTED,
        )
        .unwrap();
        let second = request_wire(
            kind.wire(),
            &dispatch(),
            &ImageBytes::default(),
            ChatSpelling::DOCUMENTED,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
            "{} writes a request that differs between two dispatches",
            kind.as_str()
        );
    }
}

#[test]
fn the_supply_layer_prepends_nothing_to_the_system_prefix() {
    let chat = dispatch();
    let handed = chat
        .system
        .iter()
        .map(|block| block.text.clone())
        .collect::<Vec<String>>()
        .join("\n\n");
    for kind in EVERY_CONNECTION {
        let sent = system_on_the_wire(kind, &chat);
        assert_eq!(
            sent,
            handed,
            "{} adds text of its own to the system prefix",
            kind.as_str()
        );
        assert!(
            !sent.contains("cch="),
            "{} carries another vendor's billing parameter",
            kind.as_str()
        );
    }
}

#[test]
fn the_first_system_block_is_the_one_the_city_put_first() {
    let chat = dispatch();
    for kind in EVERY_CONNECTION {
        assert!(
            system_on_the_wire(kind, &chat).starts_with("city norms"),
            "{} moved or shadowed the first prefix block",
            kind.as_str()
        );
    }
}
