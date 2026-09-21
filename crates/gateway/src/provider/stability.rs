// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The guard on one property of the supply layer: **under one
//! configuration, two dispatches send the same system prefix, byte for
//! byte** (roadmap 17.2).
//!
//! The property exists because of what a third-party endpoint does with
//! a prompt. A compatible relay keys its prompt cache on the whole
//! prompt text, so one byte that changes per request moves the key
//! every turn and every call pays full price for a prefix that never
//! semantically changed. An upstream client is reported to prepend a
//! per-request line carrying a `cch=` parameter to the system prompt on
//! the messages face, which its own vendor's server recognises and a
//! relay cannot: **this city must never add such a line**, and the
//! assertions below are how that stays true rather than a paragraph
//! somebody remembers.
//!
//! The assertions run no network and need no endpoint. They cover the
//! half of the path this crate owns — from the blocks handed to the
//! supply layer to the bytes leaving it. The other half, whether the
//! blocks themselves repeat across two dispatches, belongs to
//! `runtime::prefix`, which freezes them once per run.
//!
//! A deliberate exception is recorded in gateway-SPEC §8-18: changing
//! effort changes the prefix on purpose, and an intended miss is not
//! the accident guarded here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test module"
)]

use kernel::{ChatMessage, ChatRequest, ContentBlock, DialectKind, Role, SystemBlock};

use crate::dialect::{ImageBytes, request_wire};
use crate::provider::registry::{ConnectionKind, Family};

/// Every way this city connects, so a connection added without a
/// thought for caching fails here rather than on somebody's bill.
const EVERY_CONNECTION: [ConnectionKind; 7] = [
    ConnectionKind::OpenAiCompat,
    ConnectionKind::Responses,
    ConnectionKind::AnthropicNative,
    ConnectionKind::Harness(Family::Codex),
    ConnectionKind::Harness(Family::ClaudeCode),
    ConnectionKind::Harness(Family::GrokBuild),
    ConnectionKind::Harness(Family::KimiCli),
];

/// One conversation with a system prefix a person would recognise,
/// built here rather than taken from a shared sample: this file states
/// the case it guards, and a sample edited for another test would
/// quietly change what is guarded.
fn dispatch() -> ChatRequest {
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
        }],
        tools: Vec::new(),
        effort: None,
    }
}

/// The system text as it leaves for the endpoint, in either shape.
fn system_on_the_wire(kind: ConnectionKind, chat: &ChatRequest) -> String {
    let wire = request_wire(kind.wire(), chat, &ImageBytes::default()).unwrap();
    match kind.wire() {
        DialectKind::Anthropic => wire["system"]
            .as_array()
            .unwrap()
            .iter()
            .map(|block| block["text"].as_str().unwrap().to_owned())
            .collect::<Vec<String>>()
            .join("\n\n"),
        DialectKind::OpenAi => wire["messages"][0]["content"].as_str().unwrap().to_owned(),
        // The responses writer puts the system segments in the first
        // input item, as a `developer` message whose parts are the
        // segments; `instructions` is one string and would lose the
        // cache breakpoints on their edges. Read part by part, so this
        // guard still reads the bytes that leave the city.
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
        let first = request_wire(kind.wire(), &dispatch(), &ImageBytes::default()).unwrap();
        let second = request_wire(kind.wire(), &dispatch(), &ImageBytes::default()).unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
            "{} writes a request that differs between two dispatches",
            kind.as_str()
        );
    }
}

/// The assertion against the reported upstream behaviour: what reaches
/// the endpoint is what the city handed over, with nothing in front of
/// it.
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

/// The first block is where a relay's cache key starts, so a per-turn
/// value there costs the whole prefix. Stated separately from the
/// equality above because this is the sentence a reviewer checks a new
/// prefix component against.
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
