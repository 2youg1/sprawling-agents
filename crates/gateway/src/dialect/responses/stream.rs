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
        Event::Settled | Event::Unread => None,
    }
}

/// The response object the stream ended with.
///
/// The last terminal event wins, which matters for a provider that
/// revises: what a page drew from the deltas is discardable, and the
/// settled object is the record.
pub(crate) fn settled(frames: &[Value]) -> Result<Value, AxError> {
    let mut answer = None;
    for frame in frames {
        let Some(map) = frame.as_object() else {
            continue;
        };
        let Some(word) = map.get("type").and_then(Value::as_str) else {
            continue;
        };
        match Event::of(word) {
            Event::Settled => {
                if let Some(held) = map.get("response") {
                    answer = Some(held.clone());
                }
            }
            Event::TextDelta | Event::ReasoningDelta | Event::Unread => {}
        }
    }
    answer.ok_or_else(|| {
        stream_cut("the stream ended without the event that carries the settled answer")
    })
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
}
