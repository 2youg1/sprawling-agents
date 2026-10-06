// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one responses stream settles into.
//!
//! **Named events, not anonymous chunks.** Every frame carries a
//! `type`, and the three this city reads are the two text deltas and
//! the terminal event. The provider's document declares fifty-nine
//! event types in total; the rest describe built-in tools this city
//! never asks for and progress this city does not draw, and reading
//! them would be reading frames nobody acts on.
//!
//! **The stream carries its own settled answer.** The terminal event
//! holds the whole response object, so reassembly reads it rather than
//! stitching fragments together. That is what keeps one parser: a
//! streamed call and a blocking call hand `response_from` the same
//! bytes, so they cannot come to different conclusions about one reply.

use kernel::{AxError, Increment};
use serde_json::Value;

use crate::endpoint::failure::{ProviderFailure, provider_err};
use crate::mismatch::stream_cut;

/// The events this city acts on.
///
/// Named rather than matched as text at each use site, so a frame this
/// build passes over is a decision with a name rather than a wildcard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    /// A piece of the answer.
    TextDelta,
    /// A piece of the reasoning the model is showing.
    ReasoningDelta,
    /// The last event of a stream that produced an answer, whole or
    /// truncated, and the one that carries it.
    Settled,
    /// The provider failed after answering 200; the event's top-level
    /// `code` names why.
    Reported,
    /// A frame this city does not act on.
    Unread,
}

impl Event {
    fn of(word: &str) -> Event {
        match word {
            "response.output_text.delta" => Event::TextDelta,
            // Two spellings, because a model emits one or the other
            // and neither is wrong: `reasoning_text` is the reasoning
            // itself where the model shows it, and
            // `reasoning_summary_text` is the summary the rest emit.
            // Reading both here keeps every later reader from knowing
            // that.
            "response.reasoning_text.delta" | "response.reasoning_summary_text.delta" => {
                Event::ReasoningDelta
            }
            // `response.failed` carries a response object whose status
            // says the provider gave up, and `response.incomplete` one
            // that stopped at the ceiling. Both are read as settled
            // here and judged by `response_from`, which is where the
            // status is already read.
            "response.completed" | "response.incomplete" | "response.failed" => Event::Settled,
            "error" => Event::Reported,
            _ => Event::Unread,
        }
    }
}

/// What one frame carries, if it carries either stream.
pub(crate) fn increment_of(map: &serde_json::Map<String, Value>) -> Option<Increment> {
    let word = map.get("type")?.as_str()?;
    let delta = map.get("delta")?.as_str()?;
    match Event::of(word) {
        Event::TextDelta => Some(Increment::Said(delta.to_owned())),
        Event::ReasoningDelta => Some(Increment::Thought(delta.to_owned())),
        Event::Settled | Event::Reported | Event::Unread => None,
    }
}

/// The terminal response and first reported error, retained until EOF.
/// Spec: `crates/gateway/spec/Dialect/Responses.lean`, D27.
#[derive(Default)]
pub(crate) struct Stream {
    terminal: Option<Value>,
    reported: Option<AxError>,
}

impl Stream {
    pub(crate) fn retain(&mut self, mut frame: Value) {
        let Some(map) = frame.as_object_mut() else {
            return;
        };
        let Some(word) = map.get("type").and_then(Value::as_str) else {
            return;
        };
        match Event::of(word) {
            Event::Settled => {
                if let Some(response) = map.remove("response") {
                    self.terminal = Some(response);
                }
            }
            Event::Reported => {
                if self.reported.is_none() {
                    self.reported = Some(provider_err(
                        "read a streamed answer",
                        &ProviderFailure::Reported {
                            kind: map
                                .get("code")
                                .and_then(Value::as_str)
                                .unwrap_or("an error without a code"),
                        },
                    ));
                }
            }
            Event::TextDelta | Event::ReasoningDelta | Event::Unread => {}
        }
    }

    pub(crate) fn finish(self) -> Result<Value, AxError> {
        match self.reported {
            Some(error) => Err(error),
            None => self.terminal.ok_or_else(|| {
                stream_cut("the stream ended without the event that carries the settled answer")
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn retained_frames(&self) -> usize {
        usize::from(self.terminal.is_some()).saturating_add(usize::from(self.reported.is_some()))
    }
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
    use serde_json::json;

    fn settled(frames: &[Value]) -> Result<Value, AxError> {
        let mut held = crate::dialect::StreamFrames::new(kernel::DialectKind::OpenAiResponses);
        for frame in frames {
            held.retain_and_complete(frame.clone())?;
        }
        held.finish()
    }

    proptest::proptest! {
        #[test]
        fn responses_keep_the_lean_trace_properties(events in proptest::collection::vec(0u8..12, 0..80)) {
            let frames: Vec<Value> = events.into_iter().enumerate().map(|(at, event)| match event {
                0 => json!({"type": "response.completed", "response": {"status": "completed", "output": [], "usage": {"input_tokens": at, "output_tokens": 1}}}),
                1 => json!({"type": "response.incomplete", "response": {"status": "incomplete", "incomplete_details": {"reason": "max_output_tokens"}, "output": [], "usage": {"input_tokens": 1, "output_tokens": at}}}),
                2 => json!({"type": "response.failed", "response": {"status": "failed", "output": [], "usage": {}}}),
                3 => json!({"type": "response.completed"}),
                4 => json!({"type": "response.completed", "response": null}),
                5 => json!({"type": "error", "code": "server_error"}),
                6 => json!({"type": "error", "code": "invalid_prompt"}),
                7 => json!({"type": "error"}),
                8 => json!({"type": "response.output_text.delta", "delta": format!("{at}")}),
                9 => json!({"type": "response.reasoning_summary_text.delta", "delta": format!("{at}")}),
                10 => json!({"type": "response.created", "response": {"status": "in_progress"}}),
                _ => json!(["not an event"]),
            }).collect();
            let mut terminal = None;
            let mut reported = None;
            let mut increments = Vec::new();
            for frame in &frames {
                if let Some(held) = crate::dialect::increment_of(kernel::DialectKind::OpenAiResponses, frame) {
                    increments.push(held);
                }
                match frame.get("type").and_then(Value::as_str) {
                    Some("response.completed" | "response.incomplete" | "response.failed") => {
                        if let Some(response) = frame.get("response") { terminal = Some(response.clone()); }
                    }
                    Some("error") if reported.is_none() => {
                        reported = Some(provider_err("read a streamed answer", &ProviderFailure::Reported {
                            kind: frame.get("code").and_then(Value::as_str).unwrap_or("an error without a code"),
                        }));
                    }
                    _ => {}
                }
            }
            let reference = match reported {
                Some(error) => Err(error),
                None => terminal.ok_or_else(|| stream_cut("the stream ended without the event that carries the settled answer")),
            };
            let normalize = |result: Result<Value, AxError>| result.and_then(|wire| super::super::reply::response_from(&wire))
                .map_err(|error| serde_json::to_value(error).unwrap());
            proptest::prop_assert_eq!(normalize(settled(&frames)), normalize(reference));
            let mut held = crate::dialect::StreamFrames::new(kernel::DialectKind::OpenAiResponses);
            let mut forwarded = Vec::new();
            for frame in frames {
                if let Some(delta) = crate::dialect::increment_of(kernel::DialectKind::OpenAiResponses, &frame) { forwarded.push(delta); }
                held.retain_and_complete(frame).unwrap();
                proptest::prop_assert!(held.retained_frames() <= 2);
            }
            proptest::prop_assert_eq!(forwarded, increments);
        }
    }

    /// The two streams a page draws differently, told apart by the
    /// event name rather than by which field happens to be set.
    #[test]
    fn the_two_deltas_are_told_apart_by_the_name_of_the_event() {
        let said = json!({"type": "response.output_text.delta", "delta": "hello"});
        let thought = json!({"type": "response.reasoning_text.delta", "delta": "hmm"});
        let summary = json!({"type": "response.reasoning_summary_text.delta", "delta": "hm"});
        assert_eq!(
            increment_of(said.as_object().unwrap()),
            Some(Increment::Said("hello".to_owned()))
        );
        assert_eq!(
            increment_of(thought.as_object().unwrap()),
            Some(Increment::Thought("hmm".to_owned()))
        );
        assert_eq!(
            increment_of(summary.as_object().unwrap()),
            Some(Increment::Thought("hm".to_owned()))
        );
    }

    /// A frame that names an event this city does not act on produces
    /// nothing, rather than producing an empty increment a page would
    /// render as a pause.
    #[test]
    fn an_event_this_city_does_not_act_on_produces_no_increment() {
        let added = json!({"type": "response.output_item.added", "output_index": 0});
        assert_eq!(increment_of(added.as_object().unwrap()), None);
        let done = json!({"type": "response.completed", "response": {"output": []}});
        assert_eq!(increment_of(done.as_object().unwrap()), None);
    }

    #[test]
    fn the_settled_answer_is_the_object_the_terminal_event_carries() {
        let frames = vec![
            json!({"type": "response.created", "response": {"status": "in_progress"}}),
            json!({"type": "response.output_text.delta", "delta": "hi"}),
            json!({"type": "response.completed", "response": {"status": "completed"}}),
        ];
        let settled = settled(&frames).unwrap();
        assert_eq!(settled["status"], "completed");
    }

    /// A stream cut before it settled is a read error, never a
    /// shortened answer assembled from the deltas that did arrive.
    #[test]
    fn a_stream_that_never_settled_is_refused_rather_than_stitched() {
        let frames = vec![
            json!({"type": "response.created", "response": {"status": "in_progress"}}),
            json!({"type": "response.output_text.delta", "delta": "half an ans"}),
        ];
        let refused = settled(&frames).expect_err("half an answer is not an answer");
        assert_eq!(refused.code(), &kernel::AxCode::Provider);
    }

    /// The Responses stream reports a failure after 200 as an `error`
    /// event whose reason is its top-level `code`.
    #[test]
    fn an_error_event_mid_stream_is_the_failure_it_reports() {
        let reported = |code: &str| {
            let frames = vec![
                json!({"type": "response.output_text.delta", "delta": "hal"}),
                json!({"type": "error", "code": code, "message": "m", "param": null}),
            ];
            let refused = settled(&frames).expect_err("an error event is not an answer");
            (
                refused.subject().to_owned(),
                serde_json::to_value(&refused).unwrap()["retry"].clone(),
            )
        };
        assert_eq!(
            [reported("server_error"), reported("invalid_prompt")],
            [
                ("the stream reported server_error".to_owned(), json!("yes")),
                ("the stream reported invalid_prompt".to_owned(), json!("no")),
            ]
        );
    }
}
