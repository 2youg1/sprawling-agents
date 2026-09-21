// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One call whose body is read as it arrives.
//!
//! **Separated from `call` because they change for different
//! reasons.** Writing a request on the wire moves when a provider grows
//! a field; reading a body in parts moves when what "in parts" has to
//! mean changes - and it did, once, from "read it all, then walk the
//! lines" to "walk the lines as they land".

use std::io::BufRead as _;

use kernel::{AxError, ModelRequest, ModelReturn, UsdMicros};
use serde_json::Value;

use crate::cost;
use crate::dialect;

use super::config::{Endpoint, provider_err, transport_detail};

impl Endpoint {
    /// One call, with the body read as it arrives.
    ///
    /// **The answer is still the whole answer.** This asks the provider
    /// to stream, reports the text as it lands, and then reads the
    /// settled response out of the terminal frame — so what comes back
    /// is what a non-streaming call would have returned, and a stream cut
    /// halfway is a body read error rather than a shortened reply. The
    /// increments are a thing to watch; they are never the record.
    ///
    /// Two dialects carry a settled answer at the end of their stream in
    /// two shapes, and neither is worth a second parser: the request asks
    /// for a stream only when the caller wants increments, and the reply
    /// is reassembled through the same `response_from_wire` a blocking
    /// call uses.
    pub(crate) fn stream(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        let mut wire = self.wire_request(req)?;
        if let Some(map) = wire.as_object_mut() {
            map.insert("stream".to_owned(), Value::Bool(true));
            // **This wire reports no usage in a stream unless it is
            // asked to.** Without the flag every streamed call bills
            // something and accounts for nothing, so the cost page reads
            // zero for exactly the calls a person watched arrive. The
            // Anthropic stream carries its counts in `message_delta` and
            // needs no equivalent.
            if matches!(self.config.dialect, kernel::DialectKind::OpenAi) {
                map.insert(
                    "stream_options".to_owned(),
                    serde_json::json!({ "include_usage": true }),
                );
            }
        }
        let request = self.authorize(
            self.client
                .post(&self.config.base_url)
                .header("content-type", "application/json")
                .header("accept", "text/event-stream"),
        )?;
        // **A streamed call carries no deadline of its own.** The
        // transport reads the whole body under one deadline, so any
        // figure written here would cut an answer for being long; the
        // bound a person set is a silence, and [`Endpoint::frames_of`]
        // is where it is applied.
        let mut sending = request
            .json(&wire)
            .build()
            .map_err(|err| provider_err("call provider", transport_detail(&err)))?;
        *sending.timeout_mut() = None;
        let response = self
            .client
            .execute(sending)
            .map_err(|err| provider_err("call provider", transport_detail(&err)))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_err(
                "call provider",
                format!("{} answered {}", self.config.base_url, status.as_u16()),
            ));
        }
        let frames = self.frames_of(response, onto)?;
        // A cut stream is a provider failure, never a shortened reply:
        // a return is built from the settled frame, and a body that
        // ended before that frame arrived has none.
        let settled = dialect::settled_from_stream(self.config.dialect, &frames)?;
        let resp = dialect::response_from_wire(self.config.dialect, &settled)?;
        let billed: Option<UsdMicros> = match &self.config.pricing {
            Some(entry) => Some(cost::settle(&resp.usage, None, entry)?.billed),
            None => None,
        };
        ModelReturn::from_response(resp, billed)
    }

    /// Every frame the provider wrote, forwarded as it lands and given
    /// up on after one silence too long.
    ///
    /// **The body is read on a thread of its own so that a silence can
    /// be measured.** A blocking read cannot be interrupted, and the
    /// transport's own deadline covers the whole answer rather than the
    /// gaps in it; a reader beside the caller turns "nothing has
    /// arrived for this long" into a value the caller can act on, which
    /// is what the setting has said all along.
    ///
    /// The cost is named rather than hidden: when a provider goes
    /// silent and holds the connection open, the reader stays blocked
    /// on it until the provider closes it. Ending the call is what a
    /// person asked for; ending the connection is the provider's, and
    /// no answer of ours can take it away from them.
    fn frames_of(
        &self,
        response: reqwest::blocking::Response,
        onto: kernel::Increments<'_>,
    ) -> Result<Vec<Value>, AxError> {
        // The stream's own bound when it has one, and the settled
        // call's when it does not: a stream nobody bounded separately
        // is still not allowed to go quiet forever.
        let quiet_ms = self
            .config
            .stream_idle_timeout_ms
            .unwrap_or(self.config.timeout_ms);
        let quiet = std::time::Duration::from_millis(quiet_ms);
        let (lines, arriving) = std::sync::mpsc::channel::<std::io::Result<String>>();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(response).lines() {
                let broke = line.is_err();
                if lines.send(line).is_err() || broke {
                    return;
                }
            }
        });
        let mut frames = Vec::new();
        loop {
            let line = match arriving.recv_timeout(quiet) {
                Ok(Ok(line)) => line,
                Ok(Err(err)) => {
                    return Err(provider_err("read provider response", err.to_string()));
                }
                // The reader reached the end of the body and dropped
                // its end of the channel, which is how a stream ends.
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(frames),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    return Err(provider_err(
                        "read provider response",
                        format!("no byte arrived for {quiet_ms} ms"),
                    ));
                }
            };
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            // The sentinel one dialect ends with. It is not JSON, and
            // treating it as an unreadable frame would turn every
            // successful stream into a warning.
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }
            // A frame this build cannot read is skipped rather than
            // fatal: providers add event types, and a person watching
            // text arrive must not lose a call because one of them was
            // new. What cannot be skipped is the settled answer, and
            // the caller checks that.
            let Ok(frame) = serde_json::from_str::<Value>(payload) else {
                continue;
            };
            if let Some(held) = dialect::increment_of(self.config.dialect, &frame) {
                onto(&held);
            }
            frames.push(frame);
        }
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
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests {
    use super::super::config::EndpointConfig;
    use super::super::fakes::{config, fake_stream_provider, paced_stream_provider, request};
    use super::super::redemption::redemption;
    use super::*;
    use std::time::Duration;

    /// One complete answer, frame by frame, in the shape the Anthropic
    /// stream carries it.
    fn whole_answer() -> Vec<String> {
        vec![
            serde_json::json!({
                "type": "message_start",
                "message": {"usage": {"input_tokens": 1_000_000}}
            })
            .to_string(),
            serde_json::json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": "text", "text": ""}
            })
            .to_string(),
            serde_json::json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": "still "}
            })
            .to_string(),
            serde_json::json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": "writing"}
            })
            .to_string(),
            serde_json::json!({
                "type": "message_delta",
                "delta": {"stop_reason": "end_turn"},
                "usage": {"output_tokens": 0}
            })
            .to_string(),
        ]
    }

    /// **The bound is a silence, not a length.** Five frames 300 ms
    /// apart run for one and a half seconds under an 800 ms bound: a
    /// deadline would have cut this answer in half, and the model was
    /// writing the whole time. This is the case a person met as an
    /// answer truncated at five minutes while the wire called the
    /// setting an idle timeout.
    #[test]
    fn a_model_that_keeps_writing_is_never_cut_off_for_writing_at_length() {
        let (url, server) =
            paced_stream_provider(whole_answer(), Duration::from_millis(300), Duration::ZERO);
        let mut endpoint = Endpoint::new(
            EndpointConfig {
                stream_idle_timeout_ms: Some(800),
                ..config(&url)
            },
            redemption(),
        )
        .unwrap();
        let said = std::cell::RefCell::new(String::new());
        let mut onto = |held: &kernel::Increment| {
            if let kernel::Increment::Said(text) = held {
                said.borrow_mut().push_str(text);
            }
        };
        let ret = endpoint.stream(&request(), &mut onto).unwrap();
        assert_eq!(said.borrow().as_str(), "still writing");
        assert_eq!(ret.stop, Some(kernel::StopReason::EndTurn));
        server.join().unwrap();
    }

    /// A provider that stops mid-answer is given up on one bound after
    /// its last byte, and the refusal states the bound rather than the
    /// transport's own wording.
    #[test]
    fn a_stream_that_falls_silent_is_given_up_on_and_says_for_how_long() {
        let (url, server) = paced_stream_provider(
            whole_answer().into_iter().take(3).collect::<Vec<String>>(),
            Duration::from_millis(20),
            Duration::from_millis(1_500),
        );
        let mut endpoint = Endpoint::new(
            EndpointConfig {
                stream_idle_timeout_ms: Some(400),
                ..config(&url)
            },
            redemption(),
        )
        .unwrap();
        let mut onto = |_held: &kernel::Increment| {};
        let err = endpoint.stream(&request(), &mut onto).unwrap_err();
        assert_eq!(*err.code(), kernel::AxCode::Provider);
        assert!(
            err.subject().contains("no byte arrived for 400 ms"),
            "a stalled stream names the bound it passed: {}",
            err.subject()
        );
        server.join().unwrap();
    }

    /// **A stream nobody forwards is a stream in name only.** The
    /// request asked for `stream: true`, the provider answered in
    /// increments, and the reader called `.text()` - which finishes only
    /// at the end of the body. Every increment then reached the page in
    /// one burst after the model had already stopped: measured against a
    /// real provider as `model_called` at 1.9 s, twenty-five deltas in
    /// one millisecond at 11.9 s, then `model_returned`.
    ///
    /// The provider fixture answers the question itself: it will not
    /// write its closing frames until it hears that the opening ones
    /// were handed on.
    #[test]
    fn increments_reach_the_caller_while_the_body_is_still_arriving() {
        let opening = vec![
            serde_json::json!({
                "type": "message_start",
                "message": {"usage": {"input_tokens": 1_000_000}}
            })
            .to_string(),
            serde_json::json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": "text", "text": ""}
            })
            .to_string(),
            serde_json::json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": "on "}
            })
            .to_string(),
        ];
        let closing = vec![
            serde_json::json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {"type": "text_delta", "text": "it"}
            })
            .to_string(),
            serde_json::json!({
                "type": "message_delta",
                "delta": {"stop_reason": "end_turn"},
                "usage": {"output_tokens": 0}
            })
            .to_string(),
        ];
        let (saw_opening, waiting) = std::sync::mpsc::channel();
        let (url, server) = fake_stream_provider(opening, waiting, closing);
        let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();

        let said = std::cell::RefCell::new(Vec::new());
        let mut onto = |held: &kernel::Increment| {
            let kernel::Increment::Said(text) = held else {
                return;
            };
            said.borrow_mut().push(text.clone());
            // Reported once: the fixture needs to hear that one
            // increment arrived, and a closed channel afterwards is not
            // a failure of the thing under test.
            let _ = saw_opening.send(());
        };
        let ret = endpoint.stream(&request(), &mut onto).unwrap();

        assert!(
            server.join().unwrap(),
            "the provider was still writing when the first increment should have been \
             handed on; `.text()` reads to the end of the body before anything is forwarded"
        );
        assert_eq!(said.borrow().as_slice(), ["on ", "it"]);
        assert_eq!(ret.stop, Some(kernel::StopReason::EndTurn));
    }
}
