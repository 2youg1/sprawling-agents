// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The external-provider adapter: a self-written wire-format client over
//! a plain HTTP transport. One call, one HTTP
//! round trip: whether to try again is the watchdog's decision, never a
//! hidden loop here.
//!
//! Credentials resolve at the last moment: the `Sealed` value is exposed
//! only while the auth header is written, then dropped (zeroized).

//! Endpoint calls: one request, streamed or settled.

#[cfg(test)]
use kernel::UsdMicros;
use kernel::consts_policy::{IMAGE_MAX_BYTES, IMAGES_PER_TURN};
use kernel::{AxError, ChatRequest, ContentBlock, ImageRef, ModelRequest};
use serde_json::Value;

use crate::dialect::{ImageBytes, request_wire};

use super::config::{AuthSpec, Endpoint, apply_override};
use super::failure::{ProviderFailure, provider_err};
use super::header::HeaderValue;
use super::models::facts_of;
use kernel::event::record::ModelFacts;

/// Every picture one conversation refers to, in the order the blocks
/// name them: the blocks a person attached and the ones a tool produced
/// are the same value, so they are collected the same way.
pub(crate) fn pictures_in<'a>(chat: &'a ChatRequest<'_>) -> Vec<&'a ImageRef> {
    let mut found = Vec::new();
    for message in chat.messages.iter() {
        for block in &message.content {
            match block {
                ContentBlock::Image(picture) => found.push(picture),
                ContentBlock::ToolResult { attachments, .. } => found.extend(attachments.iter()),
                ContentBlock::Text { .. }
                | ContentBlock::Thinking { .. }
                | ContentBlock::RedactedThinking { .. }
                | ContentBlock::ToolUse { .. } => {}
            }
        }
    }
    found
}

/// The id one conversation carries to a host that asks for one.
///
/// Derived rather than stored: the frozen prefix and the first message
/// are the same on every turn of one conversation and differ between
/// two, so the digest of the two is stable where it has to be without
/// a second record of what a conversation is. The digest carries none
/// of the text it was taken over.
///
/// # Errors
/// `E_WIRE_MISMATCH` when the first message will not serialise.
pub(crate) fn conversation_id(req: &ModelRequest) -> Result<String, AxError> {
    let mut held = Vec::new();
    for segment in &req.segments {
        held.extend_from_slice(segment.as_bytes());
    }
    if let Some(opening) = req.chat.messages.first() {
        held.extend(
            serde_json::to_vec(opening)
                .map_err(|err| crate::mismatch::mismatch("messages[0]", &err.to_string()))?,
        );
    }
    Ok(kernel::B3Hash::digest(&held).to_string())
}

impl Endpoint {
    /// What the far side says it serves, and what it says about each.
    ///
    /// Both dialects answer `GET .../models` with `{"data":[{"id":..}]}`.
    /// What else a row carries is the vendor's own business, so each row
    /// is read by [`facts_of`] for everything it states and for
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
        let request = self.authorize(self.client.get(url))?;
        let response = request
            .send()
            .map_err(|err| provider_err("list models", &ProviderFailure::Exchange(&err)))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_err(
                "list models",
                &ProviderFailure::Refused {
                    url,
                    status,
                    headers: response.headers(),
                },
            ));
        }
        let body: Value = response
            .json()
            .map_err(|err| provider_err("read the model list", &ProviderFailure::Exchange(&err)))?;
        let rows = body.get("data").and_then(Value::as_array).ok_or_else(|| {
            provider_err(
                "read the model list",
                &ProviderFailure::Unreadable(format!("{url}: no data array")),
            )
        })?;
        let mut facts = Vec::new();
        for row in rows {
            facts.push(facts_of(row).ok_or_else(|| {
                provider_err(
                    "read the model list",
                    &ProviderFailure::Unreadable("a row has no id".to_owned()),
                )
            })?);
        }
        facts.sort_by(|left, right| left.id.cmp(&right.id));
        facts.dedup_by(|left, right| left.id == right.id);
        Ok(facts)
    }

    /// The POST one chat request leaves in: every header this endpoint
    /// adds, and the conversation's id where the host asks for one.
    ///
    /// A header the person named in `extra_headers` is theirs, so the
    /// id is not written a second time under the same name.
    ///
    /// # Errors
    /// Propagates a redemption failure and an unserialisable opening
    /// message.
    pub(super) fn chat_post(
        &self,
        req: &ModelRequest,
    ) -> Result<reqwest::blocking::RequestBuilder, AxError> {
        let request = self
            .client
            .post(&self.config.base_url)
            .header("content-type", "application/json");
        let named_by_person = |name: &str| {
            self.config
                .extra_headers
                .iter()
                .any(|(given, _)| given.eq_ignore_ascii_case(name))
        };
        let request = match crate::provider::preset::session_header(&self.config.base_url) {
            Some(name) if !named_by_person(name) => request.header(name, conversation_id(req)?),
            Some(_) | None => request,
        };
        self.authorize(request)
    }

    /// Every header this endpoint adds, written in the last slot
    /// before the wire.
    ///
    /// **Redemption happens here for both kinds of credential**: the
    /// one the registration carries and the one a person put in a
    /// header of their own. Each `Sealed` is exposed only while its
    /// header is written and dropped (zeroized) immediately after, and
    /// writing both kinds in one statement is what keeps a second,
    /// laxer path from growing beside this one.
    ///
    /// # Errors
    /// Propagates a redemption failure, naming the reference that could
    /// not be redeemed.
    pub(super) fn authorize(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, AxError> {
        let mut request = match &self.config.auth {
            AuthSpec::Bearer(reference) => {
                let sealed = (self.redemption.secrets)(reference)?;
                request.header("authorization", format!("Bearer {}", sealed.expose()))
            }
            AuthSpec::Header { name, value } => {
                let sealed = (self.redemption.secrets)(value)?;
                request.header(name, sealed.expose().as_str())
            }
            AuthSpec::None => request,
        };
        for (name, value) in &self.config.extra_headers {
            request = match value {
                HeaderValue::Plain(text) => request.header(name, text),
                HeaderValue::Redeemed(reference) => {
                    let sealed = (self.redemption.secrets)(reference)?;
                    request.header(name, sealed.expose().as_str())
                }
            };
        }
        Ok(request)
    }

    /// One POST of bytes this endpoint's own body writer did not
    /// build, answered as JSON.
    ///
    /// The audio face sends a multipart form rather than a dialect's
    /// request body. It asks this type for the transport, the deadline
    /// and the status rule, so that "how a POST leaves this city, what a
    /// non-2xx becomes, and whether the provider's body is quoted
    /// back" is answered once for every caller.
    ///
    /// # Errors
    /// `E_PROVIDER` on transport failure, on a non-success status, and
    /// on a body that is not JSON; the URL and the status are named and
    /// the provider's own body never is.
    pub(crate) fn post_bytes(&self, content_type: &str, body: Vec<u8>) -> Result<Value, AxError> {
        let url = &self.config.base_url;
        let request = self
            .authorize(
                self.client
                    .post(url)
                    .header("content-type", content_type)
                    .header("accept", "application/json"),
            )?
            .body(body);
        let response = request
            .send()
            .map_err(|err| provider_err("post to provider", &ProviderFailure::Exchange(&err)))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_err(
                "post to provider",
                &ProviderFailure::Refused {
                    url,
                    status,
                    headers: response.headers(),
                },
            ));
        }
        response
            .json()
            .map_err(|err| provider_err("read provider response", &ProviderFailure::Exchange(&err)))
    }

    /// The bytes for every picture this request refers to.
    ///
    /// Both ceilings are answered before a single byte reaches the
    /// provider, and both refusals name the limit rather than the fact
    /// that one exists.
    fn pictures_for(&self, chat: &ChatRequest) -> Result<ImageBytes, AxError> {
        let found = pictures_in(chat);
        IMAGES_PER_TURN.admit(found.len())?;
        let mut images = ImageBytes::default();
        for picture in found {
            let bytes = (self.redemption.images)(&picture.locator)?;
            IMAGE_MAX_BYTES.admit(&picture.locator, bytes.len())?;
            images.insert(&picture.locator, bytes);
        }
        Ok(images)
    }

    pub(crate) fn wire_request(&self, req: &ModelRequest) -> Result<Value, AxError> {
        let mut chat = req.chat.clone();
        chat.model = self.config.model.clone();
        let images = self.pictures_for(&chat)?;
        let mut wire = request_wire(
            self.config.dialect,
            &chat,
            &images,
            crate::provider::preset::chat_spelling(&self.config.base_url),
        )?;
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

    /// OpenCode asks for one stable id per conversation: every turn of
    /// one conversation carries the same one, and another conversation
    /// carries another.
    #[test]
    fn a_conversation_keeps_its_id_from_turn_to_turn() {
        let said = |role, text: &str| kernel::ChatMessage {
            role,
            content: vec![kernel::ContentBlock::Text {
                text: text.to_owned(),
            }],
        };
        let mut first = request();
        first
            .chat
            .messages
            .to_mut()
            .push(said(kernel::Role::User, "Task: probe"));
        let mut later = first.clone();
        later
            .chat
            .messages
            .to_mut()
            .push(said(kernel::Role::Assistant, "a later turn"));
        let mut other = request();
        other
            .chat
            .messages
            .to_mut()
            .push(said(kernel::Role::User, "another conversation"));
        let id = conversation_id(&first).unwrap();
        assert_eq!(id.len(), 64, "{id}");
        assert_eq!(conversation_id(&later).unwrap(), id);
        assert_ne!(conversation_id(&other).unwrap(), id);
    }

    /// A request carrying `count` pictures in one user message.
    fn seeing(count: u32) -> ModelRequest<'static> {
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
        req.chat.messages.to_mut().push(kernel::ChatMessage {
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
        // Every call names this city as its client.
        assert!(
            seen[0]
                .to_ascii_lowercase()
                .contains(concat!("user-agent: sprawling/", env!("CARGO_PKG_VERSION"))),
            "{}",
            seen[0]
        );
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
        // The refusal is the limit's own, whole: a caller that spelled a
        // sentence of its own would be a second authority for one limit.
        assert_eq!(err, IMAGES_PER_TURN.admit(5).unwrap_err());
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
        let request = seeing(1);
        let at = pictures_in(&request.chat)
            .first()
            .map(|picture| picture.locator.clone())
            .expect("the fixture puts one picture in the chat");
        let err = endpoint.wire_request(&request).unwrap_err();
        assert_eq!(err, IMAGE_MAX_BYTES.admit(&at, 3_000_000).unwrap_err());
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
