// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The external-provider adapter: a self-written wire-format client over
//! a plain HTTP transport. One call, one HTTP
//! round trip: retries and failover are the watchdog's and admission's
//! decisions, never a hidden loop here.
//!
//! Credentials resolve at the last moment: the `Sealed` value is exposed
//! only while the auth header is written, then dropped (zeroized).

//! Endpoint calls: one request, streamed or settled.

use kernel::{AxError, ModelRequest, ModelReturn, UsdMicros};
use serde_json::Value;

use crate::cost;
use crate::dialect;
use crate::dialect::request_wire;

use super::config::{AuthSpec, Endpoint, apply_override, provider_err, transport_detail};
impl Endpoint {
    /// What the far side says it serves.
    ///
    /// Both dialects answer `GET .../models` with `{"data":[{"id":..}]}`,
    /// and neither returns prices or token limits there — those are
    /// facts a person confirms at registration, not numbers to guess.
    ///
    /// # Errors
    /// Transport failure, a non-success status, or a body without a
    /// readable `data` array; each carries the URL that was asked.
    pub fn list_models(&self, url: &str) -> Result<Vec<String>, AxError> {
        let mut request = self.client.get(url);
        for (name, value) in &self.config.extra_headers {
            request = request.header(name, value);
        }
        request = self.authorize(request)?;
        let response = request
            .send()
            .map_err(|err| provider_err("list models", transport_detail(&err)))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_err(
                "list models",
                format!("{url} answered {}", status.as_u16()),
            ));
        }
        let body: Value = response
            .json()
            .map_err(|err| provider_err("read the model list", err.to_string()))?;
        let rows = body
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| provider_err("read the model list", format!("{url}: no data array")))?;
        let mut ids = Vec::new();
        for row in rows {
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| provider_err("read the model list", "a row has no id"))?;
            ids.push(id.to_owned());
        }
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    /// Redemption: resolve now, expose only while the header is written,
    /// drop (zeroize) immediately after.
    pub(crate) fn authorize(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, AxError> {
        Ok(match &self.config.auth {
            AuthSpec::Bearer(reference) => {
                let sealed = (self.resolver)(reference)?;
                request.header("authorization", format!("Bearer {}", sealed.expose()))
            }
            AuthSpec::Header { name, value } => {
                let sealed = (self.resolver)(value)?;
                request.header(name, sealed.expose().as_str())
            }
            _ => request,
        })
    }

    pub(crate) fn wire_request(&self, req: &ModelRequest) -> Result<Value, AxError> {
        let mut chat = req.chat.clone();
        chat.model = self.config.model.clone();
        let mut wire = request_wire(self.config.dialect, &chat)?;
        for (pointer, value) in &self.config.overrides {
            apply_override(&mut wire, pointer, value)?;
        }
        Ok(wire)
    }
}

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
        let body = response
            .text()
            .map_err(|err| provider_err("read provider response", transport_detail(&err)))?;
        let mut frames = Vec::new();
        for line in body.lines() {
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
    use super::super::config::{config, fake_provider, request, resolver};
    use super::*;
    use kernel::{AxCode, Model};
    #[test]
    fn a_success_round_trip_settles_and_maps_the_wave() {
        let body = serde_json::json!({
            "content": [
                { "type": "text", "text": "on it" },
                { "type": "tool_use", "id": "tu_9", "name": "exec", "input": {"cmd": "ls"} },
            ],
            "stop_reason": "tool_use",
            "usage": { "input_tokens": 1_000_000, "output_tokens": 0 },
        })
        .to_string();
        let (url, server) = fake_provider(vec![(200, body)], false);
        let mut endpoint = Endpoint::new(config(&url), resolver()).unwrap();
        let ret = endpoint.call(&request()).unwrap();
        assert_eq!(ret.calls.len(), 1);
        assert_eq!(ret.calls[0].id, "tu_9");
        assert_eq!(ret.billed_usd_micros, Some(UsdMicros::new(3_000_000)));
        assert_eq!(ret.stop, Some(kernel::StopReason::ToolUse));
        let seen = server.join().unwrap();
        // Auth header, extra header, override and model name all landed.
        assert!(seen[0].contains("x-api-key: sk-test-0123456789"));
        assert!(seen[0].contains("anthropic-version: 2023-06-01"));
        assert!(seen[0].contains("\"user_id\":\"city\""));
        assert!(seen[0].contains("\"model\":\"provider-model\""));
    }

    #[test]
    fn provider_status_errors_map_to_e_provider_with_status_only() {
        let (url, server) = fake_provider(vec![(429, "{}".to_owned())], false);
        let mut endpoint = Endpoint::new(config(&url), resolver()).unwrap();
        let err = endpoint.call(&request()).unwrap_err();
        assert_eq!(*err.code(), AxCode::Provider);
        assert!(err.subject().contains("429"));
        drop(server);
    }

    #[test]
    fn a_cut_body_is_e_provider_never_a_partial_return() {
        let body = serde_json::json!({
            "content": [ { "type": "text", "text": "half" } ],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 1, "output_tokens": 1 },
        })
        .to_string();
        let (url, server) = fake_provider(vec![(200, body)], true);
        let mut endpoint = Endpoint::new(config(&url), resolver()).unwrap();
        let err = endpoint.call(&request()).unwrap_err();
        assert_eq!(*err.code(), AxCode::Provider);
        drop(server);
    }

    #[test]
    fn overrides_create_missing_paths_and_refuse_non_objects() {
        let mut root = serde_json::json!({"a": 1});
        apply_override(&mut root, "/b/c", &Value::Bool(true)).unwrap();
        assert_eq!(root["b"]["c"], true);
        let err = apply_override(&mut root, "/a/c", &Value::Bool(true)).unwrap_err();
        assert_eq!(*err.code(), AxCode::ConfigInvalid);
    }
}
