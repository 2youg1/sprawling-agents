// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One call whose body is read as it arrives.
//!
//! **Separated from `call` for the reason card-4.1 separated the stream
//! readers from the request writers: they change for different
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
        }
        let mut request = self
            .client
            .post(&self.config.base_url)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream");
        for (name, value) in &self.config.extra_headers {
            request = request.header(name, value);
        }
        request = self.authorize(request)?;
        let response = request
            .json(&wire)
            .send()
            .map_err(|err| provider_err("call provider", transport_detail(&err)))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_err(
                "call provider",
                format!("{} answered {}", self.config.base_url, status.as_u16()),
            ));
        }
        // Line by line as the body arrives, never `.text()`. Reading the
        // whole body first and then walking its lines produces every
        // increment after the model has already stopped - the request
        // says `stream: true`, the provider answers in parts, and the
        // one place the parts were supposed to stay parts turned them
        // back into a document.
        let mut frames = Vec::new();
        for line in std::io::BufReader::new(response).lines() {
            let line =
                line.map_err(|err| provider_err("read provider response", err.to_string()))?;
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
            // that is checked below.
            let Ok(frame) = serde_json::from_str::<Value>(payload) else {
                continue;
            };
            if let Some(text) = dialect::increment_of(self.config.dialect, &frame) {
                onto(&text);
            }
            frames.push(frame);
        }
        let settled = dialect::settled_from_stream(self.config.dialect, &frames)?;
        let resp = dialect::response_from_wire(self.config.dialect, &settled)?;
        let billed: Option<UsdMicros> = match &self.config.pricing {
            Some(entry) => Some(cost::settle(&resp.usage, None, entry)?.billed),
            None => None,
        };
        ModelReturn::from_response(resp, billed)
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
    use super::super::fakes::{config, fake_stream_provider, request};
    use super::super::redemption::redemption;
    use super::*;

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
        let mut onto = |text: &str| {
            said.borrow_mut().push(text.to_owned());
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
