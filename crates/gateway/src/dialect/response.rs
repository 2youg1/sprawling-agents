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

//! Response dialects: frames back to ChatResponse.

use kernel::{AxCode, AxError, ChatResponse, DialectKind, Increment};
use serde_json::Value;

use crate::{anthropic, openai};

pub fn increment_of(kind: DialectKind, frame: &Value) -> Option<Increment> {
    let map = frame.as_object()?;
    match kind {
        DialectKind::Anthropic => anthropic::increment_of(map),
        DialectKind::OpenAi => openai::increment_of(map),
        _ => None,
    }
    .filter(|held| match held {
        Increment::Said(text) | Increment::Thought(text) => !text.is_empty(),
    })
}

/// The settled answer a stream ends with, in the shape a non-streaming
/// call would have returned.
///
/// **One parser for the answer.** Reassembling here rather than reading
/// the stream into a `ChatResponse` directly is what stops a second
/// authority forming: `response_from_wire` remains the only code that
/// decides what a provider said, so a streamed call and a blocking call
/// cannot come to different conclusions about the same reply.
///
/// # Errors
/// `Provider` when the stream ended without the frames that carry the
/// answer. That is the same failure a truncated body is, and it is
/// deliberately not recoverable by keeping the increments: a partial
/// reply presented as a whole one is the one outcome this must not have.
pub fn settled_from_stream(kind: DialectKind, frames: &[Value]) -> Result<Value, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::settled(frames),
        DialectKind::OpenAi => openai::settled(frames),
        _ => Err(AxError::failure(
            AxCode::EndpointDialectUnsupported,
            "read a streamed answer",
            format!("{kind:?} is not a dialect this build speaks"),
        )
        .with_recovery("attach the endpoint as anthropic or openai")),
    }
}

/// Wire response into the canonical shape.
pub fn response_from_wire(kind: DialectKind, wire: &Value) -> Result<ChatResponse, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::response_from(wire),
        DialectKind::OpenAi => openai::response_from(wire),
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

/// Canonical response onto the wire. Production uses this for replay
/// fixtures and citysim scripts; tests use it for round-trip proof.
pub fn response_wire(kind: DialectKind, resp: &ChatResponse) -> Result<Value, AxError> {
    match kind {
        DialectKind::Anthropic => anthropic::response_wire(resp),
        DialectKind::OpenAi => openai::response_wire(resp),
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
    use kernel::{ContentBlock, ModelUsage, StopReason};
    use kernel::{Payload, Tokens, ToolName};
    use proptest::prelude::*;
    use serde_json::{Map, json};
    #[test]
    fn anthropic_returns_thinking_blocks_exactly_as_it_issued_them() {
        // Official rule: "Include the complete unmodified block back to
        // the API"; altering it earns a 400 saying the thinking blocks
        // "cannot be modified". So both directions must be byte-exact.
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

        // And on the way back out, inside a request's message history.
        let mut req = sample_request();
        req.messages[1].content.insert(
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
    fn missing_usage_fields_read_as_zero() {
        let wire = json!({
            "content": [ { "type": "text", "text": "hi" } ],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 7, "output_tokens": 3 },
        });
        let resp = response_from_wire(DialectKind::Anthropic, &wire).unwrap();
        assert_eq!(resp.usage.cache_read_tokens, Tokens::new(0));
        assert_eq!(resp.usage.input_tokens, Tokens::new(7));
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
    fn usage_strategy(cache_write: bool) -> impl Strategy<Value = ModelUsage> {
        (0u64..9999, 0u64..9999, 0u64..9999, 0u64..9999).prop_map(move |(i, o, r, w)| ModelUsage {
            input_tokens: Tokens::new(i),
            output_tokens: Tokens::new(o),
            cache_read_tokens: Tokens::new(r),
            cache_write_tokens: Tokens::new(if cache_write { w } else { 0 }),
        })
    }
    fn stop_strategy() -> impl Strategy<Value = StopReason> {
        prop_oneof![
            Just(StopReason::EndTurn),
            Just(StopReason::ToolUse),
            Just(StopReason::MaxTokens),
        ]
    }

    /// A provider that answers 200 with a nulled `choices` has not spoken
    /// a shape we do not know - it has dropped the request. Found on a
    /// real hosted endpoint, where `max_tokens` above the model's own
    /// ceiling produced exactly this and the refusal that reached the
    /// person said "expected array" with an empty recovery.
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
    /// **A path without the value sends a person to curl the provider.**
    /// ModelScope repeats a streaming tool call's name as an empty
    /// string in every chunk after the first, and the refusal read
    /// `tool_calls[0].name: not a tool name` - true, and not enough to
    /// tell an empty name from a name with a space in it. Finding out
    /// which took a hand-written request to the endpoint.
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

    /// Every shape mismatch carries a way out. An empty `recovery` is
    /// the contract `AxError` states being broken in the one place a
    /// person meets it.
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
        /// Anthropic response round-trip is lossless: wire -> canonical -> wire.
        #[test]
        fn anthropic_response_roundtrip(
            blocks in proptest::collection::vec(prop_oneof![text_block(), tool_use_block()], 0..4),
            stop in stop_strategy(),
            usage in usage_strategy(true),
        ) {
            let resp = ChatResponse { content: blocks, stop, usage };
            let wire = response_wire(DialectKind::Anthropic, &resp).unwrap();
            let back = response_from_wire(DialectKind::Anthropic, &wire).unwrap();
            prop_assert_eq!(back, resp);
        }

        /// OpenAI response round-trip within its wire's expressiveness:
        /// at most one text block, no distinct cache-write slot.
        #[test]
        fn openai_response_roundtrip(
            text in proptest::option::of(text_block()),
            tools in proptest::collection::vec(tool_use_block(), 0..3),
            stop in stop_strategy(),
            usage in usage_strategy(false),
        ) {
            let mut content = Vec::new();
            if let Some(t) = text { content.push(t); }
            content.extend(tools);
            let resp = ChatResponse { content, stop, usage };
            let wire = response_wire(DialectKind::OpenAi, &resp).unwrap();
            let back = response_from_wire(DialectKind::OpenAi, &wire).unwrap();
            prop_assert_eq!(back, resp);
        }

        /// Usage integers survive both dialect wires verbatim.
        #[test]
        fn usage_is_preserved_verbatim(usage in usage_strategy(true)) {
            let resp = ChatResponse { content: vec![], stop: StopReason::EndTurn, usage };
            let wire = response_wire(DialectKind::Anthropic, &resp).unwrap();
            prop_assert_eq!(wire["usage"]["input_tokens"].as_u64().unwrap(), usage.input_tokens.get());
            prop_assert_eq!(wire["usage"]["cache_creation_input_tokens"].as_u64().unwrap(), usage.cache_write_tokens.get());
            let back = response_from_wire(DialectKind::Anthropic, &wire).unwrap();
            prop_assert_eq!(back.usage, usage);
        }
    }
}
