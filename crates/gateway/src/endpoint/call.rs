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

#[cfg(test)]
use kernel::UsdMicros;
use kernel::consts_policy::{IMAGE_MAX_BYTES, IMAGES_PER_TURN};
use kernel::{AxCode, AxError, ChatRequest, ContentBlock, ImageRef, Locator, ModelRequest};
use serde_json::Value;

use crate::dialect::{ImageBytes, request_wire};

use super::config::{AuthSpec, Endpoint, apply_override, provider_err, transport_detail};
use super::models::ModelFacts;

/// Every picture one conversation refers to, in the order the blocks
/// name them: the blocks a person attached and the ones a tool produced
/// are the same value, so they are collected the same way.
pub(crate) fn pictures_in(chat: &ChatRequest) -> Vec<&ImageRef> {
    let mut found = Vec::new();
    for message in &chat.messages {
        for block in &message.content {
            match block {
                ContentBlock::Image(picture) => found.push(picture),
                ContentBlock::ToolResult { attachments, .. } => found.extend(attachments.iter()),
                _ => {}
            }
        }
    }
    found
}

fn too_many(found: usize) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "put pictures on a provider request",
        format!("this turn carries {found} pictures"),
    )
    .with_recovery(format!(
        "one turn carries at most {IMAGES_PER_TURN} pictures; \
         send the rest in a later turn"
    ))
}

fn too_large(at: &Locator, size: usize) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "put a picture on a provider request",
        format!("{at} is {size} bytes"),
    )
    .with_recovery(format!(
        "one picture is at most {IMAGE_MAX_BYTES} bytes; \
         shrink it before attaching it"
    ))
}
impl Endpoint {
    /// What the far side says it serves, and what it says about each.
    ///
    /// Both dialects answer `GET .../models` with `{"data":[{"id":..}]}`.
    /// What else a row carries is the vendor's own business, so each row
    /// is read by [`ModelFacts::read`] for everything it states and for
    /// nothing it does not; a row this city cannot name is left out
    /// rather than given an invented one.
    ///
    /// Rows come back ordered by id and without repeats, so a person
    /// reading the table twice reads it in the same order.
    ///
    /// # Errors
    /// Transport failure, a non-success status, or a body without a
    /// readable `data` array; each carries the URL that was asked.
    pub fn list_models(&self, url: &str) -> Result<Vec<ModelFacts>, AxError> {
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
        let mut facts = Vec::new();
        for row in rows {
            facts.push(
                ModelFacts::read(row)
                    .ok_or_else(|| provider_err("read the model list", "a row has no id"))?,
            );
        }
        facts.sort_by(|left, right| left.id.cmp(&right.id));
        facts.dedup_by(|left, right| left.id == right.id);
        Ok(facts)
    }

    /// Redemption: resolve now, expose only while the header is written,
    /// drop (zeroize) immediately after.
    pub(crate) fn authorize(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, AxError> {
        Ok(match &self.config.auth {
            AuthSpec::Bearer(reference) => {
                let sealed = (self.redemption.secrets)(reference)?;
                request.header("authorization", format!("Bearer {}", sealed.expose()))
            }
            AuthSpec::Header { name, value } => {
                let sealed = (self.redemption.secrets)(value)?;
                request.header(name, sealed.expose().as_str())
            }
            _ => request,
        })
    }

    /// The bytes for every picture this request refers to.
    ///
    /// Both ceilings are answered before a single byte reaches the
    /// provider, and both refusals name the limit rather than the fact
    /// that one exists.
    fn pictures_for(&self, chat: &ChatRequest) -> Result<ImageBytes, AxError> {
        let found = pictures_in(chat);
        if u32::try_from(found.len()).unwrap_or(u32::MAX) > IMAGES_PER_TURN {
            return Err(too_many(found.len()));
        }
        let mut images = ImageBytes::default();
        for picture in found {
            let bytes = (self.redemption.images)(&picture.locator)?;
            if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > IMAGE_MAX_BYTES {
                return Err(too_large(&picture.locator, bytes.len()));
            }
            images.insert(&picture.locator, bytes);
        }
        Ok(images)
    }

    pub(crate) fn wire_request(&self, req: &ModelRequest) -> Result<Value, AxError> {
        let mut chat = req.chat.clone();
        chat.model = self.config.model.clone();
        let images = self.pictures_for(&chat)?;
        let mut wire = request_wire(self.config.dialect, &chat, &images)?;
        for (pointer, value) in &self.config.overrides {
            apply_override(&mut wire, pointer, value)?;
        }
        Ok(wire)
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
    use super::super::fakes::{config, fake_provider, request};
    use super::super::redemption::{Redemption, redemption, resolver};
    use super::*;
    use kernel::{AxCode, Model};

    /// A request carrying `count` pictures in one user message.
    fn seeing(count: u32) -> ModelRequest {
        let mut req = request();
        let mut content = Vec::new();
        for i in 0..count {
            let hex = format!("{i:02x}").repeat(32);
            content.push(kernel::ContentBlock::Image(kernel::ImageRef {
                locator: kernel::Locator::parse(&format!("cas:b3-{hex}")).unwrap(),
                media_type: kernel::ImageType::Png,
                width: 8,
                height: 8,
            }));
        }
        req.chat.messages.push(kernel::ChatMessage {
            role: kernel::Role::User,
            content,
        });
        req
    }
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
        let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
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
        let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
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
        let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
        let err = endpoint.call(&request()).unwrap_err();
        assert_eq!(*err.code(), AxCode::Provider);
        drop(server);
    }

    #[test]
    fn the_fifth_picture_in_one_turn_is_refused_before_any_byte_is_sent() {
        let endpoint =
            Endpoint::new(config("http://127.0.0.1:1/v1/messages"), redemption()).unwrap();
        let err = endpoint.wire_request(&seeing(5)).unwrap_err();
        assert_eq!(*err.code(), AxCode::InvalidArgs);
        assert!(
            err.recovery().contains("4"),
            "a refusal that will not say the limit cannot be acted on: {}",
            err.recovery()
        );
        // Four is still a turn this endpoint will carry.
        assert!(endpoint.wire_request(&seeing(4)).is_ok());
    }

    #[test]
    fn a_picture_larger_than_the_ceiling_is_refused_with_the_ceiling_named() {
        let endpoint = Endpoint::new(
            config("http://127.0.0.1:1/v1/messages"),
            Redemption::new(
                resolver(),
                std::sync::Arc::new(|_at: &kernel::Locator| Ok(vec![0u8; 3_000_000])),
            ),
        )
        .unwrap();
        let err = endpoint.wire_request(&seeing(1)).unwrap_err();
        assert_eq!(*err.code(), AxCode::InvalidArgs);
        assert!(err.recovery().contains("2097152"), "{}", err.recovery());
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
