// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, ChatResponse, DialectKind, Increment, ToolCall};
use serde_json::Value;

use super::responses;
use crate::{anthropic, openai};

pub fn increment_of(kind: DialectKind, frame: &Value) -> Option<Increment> {
    let map = frame.as_object()?;
    match kind {
        DialectKind::Anthropic => anthropic::increment_of(map),
        DialectKind::OpenAi => openai::increment_of(map),
        DialectKind::OpenAiResponses => responses::increment_of(map),
    }
    .filter(|held| match held {
        Increment::Said(text) | Increment::Thought(text) => !text.is_empty(),
    })
}

pub(crate) struct StreamFrames {
    kind: DialectKind,
    frames: Vec<Value>,
}

impl StreamFrames {
    pub(crate) fn new(kind: DialectKind) -> Self {
        Self {
            kind,
            frames: Vec::new(),
        }
    }

    pub(crate) fn retain_and_complete(
        &mut self,
        frame: Value,
    ) -> Result<Option<ToolCall>, AxError> {
        self.frames.push(frame);
        match self.kind {
            DialectKind::Anthropic => anthropic::call_completed_by(&self.frames),
            DialectKind::OpenAi | DialectKind::OpenAiResponses => Ok(None),
        }
    }

    pub(crate) fn finish(self) -> Result<Value, AxError> {
        match self.kind {
            DialectKind::Anthropic => anthropic::settled(&self.frames),
            DialectKind::OpenAi => openai::settled(&self.frames),
            DialectKind::OpenAiResponses => responses::settled(&self.frames),
        }
    }

    #[cfg(test)]
    pub(crate) fn retained_frames(&self) -> usize {
        self.frames.len()
    }
}

pub fn response_from_wire(kind: DialectKind, wire: &Value) -> Result<ChatResponse, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::response_from(wire),
        DialectKind::OpenAi => openai::response_from(wire),
        DialectKind::OpenAiResponses => responses::response_from(wire),
    }
}

#[cfg(test)]
pub(crate) fn response_wire(kind: DialectKind, resp: &ChatResponse) -> Result<Value, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::response_wire(resp),
        DialectKind::OpenAi => openai::response_wire(resp),
        DialectKind::OpenAiResponses => responses::response_wire(resp),
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
    use super::super::request::{request_wire, sample_request};
    use super::*;
    use kernel::{AxCode, CacheCount, ContentBlock, ModelUsage, StopReason};
    use kernel::{Payload, Tokens, ToolName};
    use proptest::prelude::*;
    use serde_json::{Map, json};
    #[test]
    fn anthropic_returns_thinking_blocks_exactly_as_it_issued_them() {
        let wire = json!({
            "content": [
                { "type": "thinking", "thinking": "two parts", "signature": "WaUjzkyp" },
                { "type": "redacted_thinking", "data": "EroBCkYIARgC" },
                { "type": "text", "text": "answer" },
            ],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 1, "output_tokens": 2 },
        });
        let canonical = response_from_wire(DialectKind::Anthropic, &wire).unwrap();
        assert_eq!(
            canonical.content[0],
            ContentBlock::Thinking {
                thinking: "two parts".to_owned(),
                signature: "WaUjzkyp".to_owned(),
            }
        );
        assert_eq!(
            canonical.content[1],
            ContentBlock::RedactedThinking {
                data: "EroBCkYIARgC".to_owned(),
            }
        );
        assert_eq!(
            response_wire(DialectKind::Anthropic, &canonical).unwrap()["content"],
            wire["content"],
            "a thinking block that made a round trip is no longer the one Claude signed"
        );

        let mut req = sample_request();
        req.messages.to_mut()[1].content.insert(
            0,
            ContentBlock::Thinking {
                thinking: "two parts".to_owned(),
                signature: "WaUjzkyp".to_owned(),
            },
        );
        let out = request_wire(
            DialectKind::Anthropic,
            &req,
            &crate::dialect::ImageBytes::default(),
            crate::provider::preset::ChatSpelling::DOCUMENTED,
        )
        .unwrap();
        assert_eq!(out["messages"][1]["content"][0], wire["content"][0]);
    }

    #[test]
    fn unknown_stop_reason_is_a_mismatch_not_a_guess() {
        let wire = json!({
            "content": [], "stop_reason": "pause_turn",
            "usage": { "input_tokens": 0, "output_tokens": 0 },
        });
        let err = response_from_wire(DialectKind::Anthropic, &wire).unwrap_err();
        assert_eq!(*err.code(), AxCode::WireMismatch);
        assert!(err.subject().contains("pause_turn"));
    }

    #[test]
    fn missing_cache_counts_read_as_unreported() {
        let wire = json!({
            "content": [ { "type": "text", "text": "hi" } ],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 7, "output_tokens": 3 },
        });
        let resp = response_from_wire(DialectKind::Anthropic, &wire).unwrap();
        assert_eq!(
            (resp.usage.cache_read_tokens, resp.usage.cache_write_tokens),
            (CacheCount::Unreported, CacheCount::Unreported)
        );
        assert_eq!(resp.usage.input_tokens, Tokens::new(7));
        let responses = json!({
            "output": [], "status": "completed",
            "usage": { "input_tokens": 7, "output_tokens": 3 },
        });
        let chat = json!({
            "choices": [ { "message": { "role": "assistant", "content": "hi" }, "finish_reason": "stop" } ],
            "usage": { "prompt_tokens": 7, "completion_tokens": 3,
                       "prompt_tokens_details": { "cached_tokens": 0 } },
        });
        let read = |dialect, wire: &Value| {
            let usage = response_from_wire(dialect, wire).unwrap().usage;
            (usage.cache_read_tokens, usage.cache_write_tokens)
        };
        assert_eq!(
            [
                read(DialectKind::OpenAiResponses, &responses),
                read(DialectKind::OpenAi, &chat),
            ],
            [
                (CacheCount::Unreported, CacheCount::Unreported),
                (CacheCount::Reported(Tokens::new(0)), CacheCount::Unreported),
            ]
        );
    }

    #[test]
    fn every_dialect_counts_the_whole_prompt_as_input() {
        let anthropic = json!({
            "content": [],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 10, "output_tokens": 1,
                       "cache_read_input_tokens": 90, "cache_creation_input_tokens": 5 },
        });
        let openai = json!({
            "choices": [ { "message": { "role": "assistant", "content": "hi" },
                           "finish_reason": "stop" } ],
            "usage": { "prompt_tokens": 105, "completion_tokens": 1,
                       "prompt_tokens_details": { "cached_tokens": 90 } },
        });
        let input = |kind, wire| response_from_wire(kind, wire).unwrap().usage.input_tokens;
        assert_eq!(
            [
                input(DialectKind::Anthropic, &anthropic),
                input(DialectKind::OpenAi, &openai)
            ],
            [Tokens::new(105), Tokens::new(105)]
        );
    }
    fn text_block() -> impl Strategy<Value = ContentBlock> {
        "[a-z ]{1,20}".prop_map(|text| ContentBlock::Text { text })
    }
    fn tool_use_block() -> impl Strategy<Value = ContentBlock> {
        ("[a-z]{1,8}", "[a-z_]{1,10}", "[a-z]{0,12}").prop_map(|(id, name, arg)| {
            let mut args = Map::new();
            args.insert("q".to_owned(), Value::String(arg));
            ContentBlock::ToolUse {
                id,
                name: ToolName::parse(&name).unwrap(),
                input: Payload::new(args).unwrap(),
            }
        })
    }

    fn usage_strategy(dialect: DialectKind) -> impl Strategy<Value = ModelUsage> {
        (0u64..9999, 0u64..9999, 0u64..9999, 0u64..9999).prop_map(move |(i, o, r, w)| {
            let write = if dialect == DialectKind::Anthropic {
                w
            } else {
                0
            };

            let cache_write_tokens = match dialect {
                DialectKind::OpenAi => CacheCount::Unreported,
                DialectKind::Anthropic | DialectKind::OpenAiResponses => {
                    CacheCount::Reported(Tokens::new(write))
                }
            };
            ModelUsage {
                input_tokens: Tokens::new(i + r + write),
                output_tokens: Tokens::new(o),
                cache_read_tokens: CacheCount::Reported(Tokens::new(r)),
                cache_write_tokens,
                dialect: Some(dialect),
            }
        })
    }
    fn stop_strategy() -> impl Strategy<Value = StopReason> {
        prop_oneof![
            Just(StopReason::EndTurn),
            Just(StopReason::ToolUse),
            Just(StopReason::MaxTokens),
        ]
    }

    #[test]
    fn a_dropped_request_is_told_apart_from_a_shape_we_cannot_read() {
        let dropped = serde_json::json!({
            "id": "", "object": "", "created": 0, "model": "m",
            "choices": null,
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        });
        let refused =
            super::super::response::response_from_wire(kernel::DialectKind::OpenAi, &dropped)
                .expect_err("a nulled answer is not an answer");
        assert_eq!(refused.code(), &kernel::AxCode::Provider);
        assert!(
            refused.recovery().contains("max output tokens"),
            "a refusal has to name what to change: {}",
            refused.recovery()
        );
    }

    #[test]
    fn a_name_this_dialect_cannot_read_is_quoted_in_the_refusal() {
        for (found, quoted) in [("", "\"\""), ("two words", "\"two words\"")] {
            let wire = serde_json::json!({
                "choices": [{"message": {"role": "assistant", "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": {"name": found, "arguments": "{}"}
                }]}}],
                "usage": {}
            });
            let refused =
                super::super::response::response_from_wire(kernel::DialectKind::OpenAi, &wire)
                    .expect_err("a name the tool grammar refuses is not a call");
            assert!(
                refused.subject().contains(quoted),
                "the refusal has to name what arrived, not only where: {}",
                refused.subject()
            );
        }
    }

    #[test]
    fn no_translation_refusal_leaves_a_person_with_nothing_to_try() {
        let alien = serde_json::json!({ "choices": "not an array", "usage": {} });
        let refused =
            super::super::response::response_from_wire(kernel::DialectKind::OpenAi, &alien)
                .expect_err("a string is not a choices array");
        assert!(!refused.recovery().is_empty());
    }
    #[test]
    fn float_tool_input_is_refused_at_the_wire_face() {
        let wire = json!({
            "content": [ { "type": "tool_use", "id": "x", "name": "exec",
                           "input": { "temperature": 0.5 } } ],
            "stop_reason": "tool_use",
            "usage": { "input_tokens": 1, "output_tokens": 1 },
        });
        let err = response_from_wire(DialectKind::Anthropic, &wire).unwrap_err();
        assert_eq!(*err.code(), AxCode::WireMismatch);
    }

    proptest! {

        #[test]
        fn anthropic_response_roundtrip(
            blocks in proptest::collection::vec(prop_oneof![text_block(), tool_use_block()], 0..4),
            stop in stop_strategy(),
            usage in usage_strategy(DialectKind::Anthropic),
        ) {
            let resp = ChatResponse { content: blocks, stop, usage };
            let wire = response_wire(DialectKind::Anthropic, &resp).unwrap();
            let back = response_from_wire(DialectKind::Anthropic, &wire).unwrap();
            prop_assert_eq!(back, resp);
        }

        #[test]
        fn openai_response_roundtrip(
            text in proptest::option::of(text_block()),
            tools in proptest::collection::vec(tool_use_block(), 0..3),
            stop in stop_strategy(),
            usage in usage_strategy(DialectKind::OpenAi),
        ) {
            let mut content = Vec::new();
            if let Some(t) = text { content.push(t); }
            content.extend(tools);
            let resp = ChatResponse { content, stop, usage };
            let wire = response_wire(DialectKind::OpenAi, &resp).unwrap();
            let back = response_from_wire(DialectKind::OpenAi, &wire).unwrap();
            prop_assert_eq!(back, resp);
        }

        #[test]
        fn usage_is_preserved_verbatim(usage in usage_strategy(DialectKind::Anthropic)) {
            let resp = ChatResponse { content: vec![], stop: StopReason::EndTurn, usage };
            let wire = response_wire(DialectKind::Anthropic, &resp).unwrap();
            let uncached = usage.input_tokens.get() - usage.cache_read_tokens.or_zero().get() - usage.cache_write_tokens.or_zero().get();
            prop_assert_eq!(wire["usage"]["input_tokens"].as_u64().unwrap(), uncached);
            prop_assert_eq!(wire["usage"]["cache_creation_input_tokens"].as_u64().unwrap(), usage.cache_write_tokens.or_zero().get());
            let back = response_from_wire(DialectKind::Anthropic, &wire).unwrap();
            prop_assert_eq!(back.usage, usage);
        }
    }
}
