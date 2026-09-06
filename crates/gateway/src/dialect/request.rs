// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which dialect answers a question, and nothing about how it answers.
//!
//! Five entrances, one `match` each, and a closed set of two: a dialect
//! this build cannot translate is refused rather than approximated with
//! the nearer of the two it knows. What each dialect does with a request
//! is `gateway::anthropic`'s and `gateway::openai`'s; what they share is
//! `gateway::mismatch`'s.
//!
//! The canonical conversation is Anthropic-shaped, so the losses are all
//! on the other path and each is documented where it is taken.
//!
//! No I/O, no state, no clock: byte-for-byte explainable requests are
//! the whole point of writing the wire format ourselves.

//! Request dialects: one ChatRequest, each wire.

use kernel::{AxCode, AxError, ChatRequest, DialectKind};
use serde_json::Value;

use crate::{anthropic, openai};

pub fn request_wire(kind: DialectKind, req: &ChatRequest) -> Result<Value, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::request(req),
        DialectKind::OpenAi => openai::request(req),
        // Fail closed: a dialect this build cannot translate is not
        // approximated with the nearer of the two it knows.
        _ => Err(AxError::failure(
            AxCode::EndpointDialectUnsupported,
            "translate wire",
            format!("{kind:?} is not a dialect this build speaks"),
        )
        .with_recovery("attach the endpoint as anthropic or openai")),
    }
}
#[cfg(test)]
use kernel::{ChatMessage, ContentBlock, Payload, Role, SystemBlock, ToolDef, ToolName};
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
pub(crate) fn sample_request() -> ChatRequest {
    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::String("object".to_owned()));
    let mut args = Map::new();
    args.insert("path".to_owned(), Value::String("notes.md".to_owned()));
    ChatRequest {
        model: "sonnet".to_owned(),
        max_tokens: 4096,
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
                }],
            },
        ],
        tools: vec![ToolDef {
            name: ToolName::parse("exec").unwrap(),
            description: "run things".to_owned(),
            input_schema: Payload::new(schema).unwrap(),
        }],
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
    use super::super::request::sample_request;
    use super::*;
    use kernel::{ContentBlock, Effort};

    #[test]
    fn anthropic_request_wire_is_pinned() {
        let wire = request_wire(DialectKind::Anthropic, &sample_request()).unwrap();
        insta::assert_snapshot!(serde_json::to_string_pretty(&wire).unwrap());
    }

    #[test]
    fn openai_request_wire_is_pinned() {
        let wire = request_wire(DialectKind::OpenAi, &sample_request()).unwrap();
        insta::assert_snapshot!(serde_json::to_string_pretty(&wire).unwrap());
    }

    #[test]
    fn anthropic_breakpoints_land_on_marked_blocks_only() {
        let wire = request_wire(DialectKind::Anthropic, &sample_request()).unwrap();
        let system = wire["system"].as_array().unwrap();
        assert!(
            system
                .iter()
                .all(|b| b["cache_control"]["type"] == "ephemeral")
        );
        let mut unmarked = sample_request();
        unmarked.system[1].cache = false;
        let wire = request_wire(DialectKind::Anthropic, &unmarked).unwrap();
        assert!(wire["system"][1].get("cache_control").is_none());
    }

    #[test]
    fn tool_shapes_survive_both_request_dialects() {
        let req = sample_request();
        let anthropic = request_wire(DialectKind::Anthropic, &req).unwrap();
        assert_eq!(anthropic["tools"][0]["name"], "exec");
        assert_eq!(anthropic["tools"][0]["input_schema"]["type"], "object");
        let openai = request_wire(DialectKind::OpenAi, &req).unwrap();
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
        req.messages[1].content.insert(
            0,
            ContentBlock::Thinking {
                thinking: "two parts".to_owned(),
                signature: "WaUjzkyp".to_owned(),
            },
        );
        let out = request_wire(DialectKind::OpenAi, &req).unwrap();
        assert!(
            !out.to_string().contains("WaUjzkyp"),
            "a signature the other provider cannot verify does not belong on its wire"
        );
        assert!(out.to_string().contains("reading"));
    }

    #[test]
    fn effort_rides_the_wire_each_dialect_spells_it_its_own_way() {
        let mut req = sample_request();
        assert!(
            request_wire(DialectKind::Anthropic, &req)
                .unwrap()
                .get("effort")
                .is_none(),
            "an unstated effort writes no field: the provider's default is its own business"
        );

        req.effort = Some(Effort::High);
        assert_eq!(
            request_wire(DialectKind::Anthropic, &req).unwrap()["effort"],
            "high"
        );
        assert_eq!(
            request_wire(DialectKind::OpenAi, &req).unwrap()["reasoning"]["effort"],
            "high"
        );

        // The one place the two dialects part: not thinking is an effort
        // value on one wire and a different field on the other.
        req.effort = Some(Effort::None);
        let anthropic = request_wire(DialectKind::Anthropic, &req).unwrap();
        assert_eq!(anthropic["thinking"]["type"], "disabled");
        assert!(anthropic.get("effort").is_none());
        assert_eq!(
            request_wire(DialectKind::OpenAi, &req).unwrap()["reasoning"]["effort"],
            "none"
        );
    }

    #[test]
    fn every_level_of_the_ladder_reaches_both_wires() {
        let mut req = sample_request();
        for (level, spelling) in [
            (Effort::Low, "low"),
            (Effort::Medium, "medium"),
            (Effort::High, "high"),
            (Effort::XHigh, "xhigh"),
            (Effort::Max, "max"),
        ] {
            req.effort = Some(level);
            assert_eq!(
                request_wire(DialectKind::Anthropic, &req).unwrap()["effort"],
                spelling
            );
            assert_eq!(
                request_wire(DialectKind::OpenAi, &req).unwrap()["reasoning"]["effort"],
                spelling
            );
        }
    }
}
