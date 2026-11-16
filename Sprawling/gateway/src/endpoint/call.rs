// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[cfg(test)]
use kernel::UsdMicros;
use kernel::consts_policy::{IMAGE_MAX_BYTES, IMAGES_PER_TURN};
use kernel::{AxError, ChatRequest, ContentBlock, DialectKind, ImageRef, ModelRequest};
use serde_json::Value;

use crate::dialect::{ImageBytes, request_wire};

use super::config::{AuthSpec, Endpoint, apply_override};
use super::failure::{ProviderFailure, provider_err};
use super::header::HeaderValue;
use super::models::facts_of;
use kernel::event::record::ModelFacts;

const MODEL_LIST_PAGES: usize = 16;

pub(crate) fn pictures_in<'a>(chat: &'a ChatRequest<'_>) -> Vec<&'a ImageRef> {
    let mut found = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for picture in picture_parts(chat) {
        if seen.insert(picture.locator.to_string()) {
            found.push(picture);
        }
    }
    found
}

fn picture_parts<'a>(chat: &'a ChatRequest<'_>) -> impl Iterator<Item = &'a ImageRef> {
    chat.messages
        .iter()
        .flat_map(|message| &message.content)
        .flat_map(|block| {
            let pictures: &[ImageRef] = match block {
                ContentBlock::Image(picture) => std::slice::from_ref(picture),
                ContentBlock::ToolResult { attachments, .. } => attachments,
                ContentBlock::Text { .. }
                | ContentBlock::Thinking { .. }
                | ContentBlock::RedactedThinking { .. }
                | ContentBlock::ToolUse { .. } => &[],
            };
            pictures.iter()
        })
}

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
    pub fn list_models(&self, url: &str) -> Result<Vec<ModelFacts>, AxError> {
        let mut facts = Vec::new();
        let mut after: Option<String> = None;
        for _ in 0..MODEL_LIST_PAGES {
            let asked = match (self.config.dialect, &after) {
                (DialectKind::Anthropic, None) => format!("{url}?limit=1000"),
                (DialectKind::Anthropic, Some(last)) => format!("{url}?limit=1000&after_id={last}"),
                (DialectKind::OpenAi | DialectKind::OpenAiResponses, _) => url.to_owned(),
            };
            let body = self.model_page(&asked)?;
            let rows = body.get("data").and_then(Value::as_array).ok_or_else(|| {
                provider_err(
                    "read the model list",
                    &ProviderFailure::Unreadable(format!("{url}: no data array")),
                )
            })?;
            for row in rows {
                facts.push(facts_of(row).ok_or_else(|| {
                    provider_err(
                        "read the model list",
                        &ProviderFailure::Unreadable("a row has no id".to_owned()),
                    )
                })?);
            }
            after = match (self.config.dialect, body.get("has_more")) {
                (DialectKind::Anthropic, Some(Value::Bool(true))) => body
                    .get("last_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                (
                    DialectKind::Anthropic | DialectKind::OpenAi | DialectKind::OpenAiResponses,
                    _,
                ) => None,
            };
            if after.is_none() {
                break;
            }
        }
        facts.sort_by(|left, right| left.id.cmp(&right.id));
        facts.dedup_by(|left, right| left.id == right.id);
        Ok(facts)
    }

    fn model_page(&self, url: &str) -> Result<Value, AxError> {
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
        response
            .json()
            .map_err(|err| provider_err("read the model list", &ProviderFailure::Exchange(&err)))
    }

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

    pub(super) fn authorize(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, AxError> {
        let mut request = match &self.config.auth {
            AuthSpec::Bearer(reference) => {
                let sealed = self.redemption.account_credential(reference)?;
                request.header("authorization", format!("Bearer {}", sealed.expose()))
            }
            AuthSpec::Header { name, value } => {
                let sealed = self.redemption.account_credential(value)?;
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

    fn pictures_for(&self, chat: &ChatRequest) -> Result<ImageBytes, AxError> {
        let found = pictures_in(chat);
        IMAGES_PER_TURN.admit(found.len())?;
        let max_side = crate::provider::preset::image_max_side(
            &self.config.base_url,
            &chat.model,
            picture_parts(chat).count(),
        );
        let mut images = ImageBytes::default();
        for picture in found {
            let bytes = (self.redemption.images)(&picture.locator)?;
            IMAGE_MAX_BYTES.admit(&picture.locator, bytes.len())?;
            images.insert(picture, bytes, max_side)?;
        }
        Ok(images)
    }

    fn cache_key_field(&self) -> Option<&'static str> {
        match self.config.dialect {
            DialectKind::OpenAi | DialectKind::OpenAiResponses => {
                crate::provider::preset::cache_key_field(&self.config.base_url)
            }
            DialectKind::Anthropic => None,
        }
    }

    pub(crate) fn wire_request(&self, req: &ModelRequest) -> Result<Value, AxError> {
        let mut chat = req.chat.clone();
        chat.model = self.config.model.clone();
        let images = self.pictures_for(&chat)?;
        let spelling = crate::provider::preset::chat_spelling(&self.config.base_url);
        let mut wire = request_wire(self.config.dialect, &chat, &images, spelling)?;

        if let (Some(ask), Value::Object(root)) = (self.thinking.ask(chat.effort), &mut wire) {
            for (field, value) in crate::provider::thinking::encoding::fields(
                ask,
                self.config.dialect,
                spelling.effort,
            ) {
                root.insert(field.to_owned(), value);
            }
        }

        if let Value::Object(root) = &mut wire {
            for (field, value) in
                crate::provider::generation::request_values(&self.config.model, &chat.generation)?
            {
                root.insert(field.to_owned(), value);
            }
        }

        if let Some(field) = self.cache_key_field() {
            apply_override(
                &mut wire,
                &format!("/{field}"),
                &Value::String(conversation_id(req)?),
            )?;
        }
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
    use kernel::AxCode;

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

    fn deepseek_picture(
        bytes: Vec<u8>,
        media_type: kernel::ImageType,
        width: u32,
        height: u32,
    ) -> (Endpoint, ModelRequest<'static>) {
        let picture = kernel::ImageRef {
            locator: kernel::Locator::cas(kernel::B3Hash::digest(&bytes)),
            media_type,
            width,
            height,
        };
        let mut called = request();
        called.chat.messages.to_mut().push(kernel::ChatMessage {
            role: kernel::Role::User,
            content: vec![ContentBlock::Image(picture)],
        });
        let mut configured = config("https://api.deepseek.com");
        configured.dialect = DialectKind::OpenAi;
        configured.model = "deepseek-flash".to_owned();
        configured.pricing = None;
        let endpoint = Endpoint::new(
            configured,
            Redemption::new(
                resolver(),
                std::sync::Arc::new(move |_at: &kernel::Locator| Ok(bytes.clone())),
            ),
        )
        .unwrap();
        (endpoint, called)
    }

    fn wire_picture(wire: &Value) -> Vec<u8> {
        use base64::Engine as _;
        let url = wire["messages"][0]["content"][0]["image_url"]["url"]
            .as_str()
            .unwrap();
        let (_, encoded) = url.split_once(',').unwrap();
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap()
    }

    #[test]
    fn a_deepseek_picture_over_the_side_ceiling_is_resized_at_the_request_edge() {
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgb8(1356, 16384)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        let (endpoint, called) = deepseek_picture(bytes, kernel::ImageType::Jpeg, 1356, 16384);

        let wire = endpoint.wire_request(&called).unwrap();
        let sent = wire_picture(&wire);
        let dimensions = image::ImageReader::new(std::io::Cursor::new(sent))
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap();
        assert_eq!(dimensions, (678, 8192));
    }

    #[test]
    fn understated_picture_dimensions_cannot_bypass_the_ceiling() {
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgb8(8, 9000)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let (endpoint, mut called) = deepseek_picture(bytes, kernel::ImageType::Png, 1, 1);
        let ContentBlock::Image(mut second) = called.chat.messages[0].content[0].clone() else {
            panic!("fixture must contain a picture");
        };
        second.width = 8;
        second.height = 9000;
        called.chat.messages.to_mut()[0]
            .content
            .push(ContentBlock::Image(second));
        let wire = endpoint.wire_request(&called).unwrap();
        let sent = image::load_from_memory(&wire_picture(&wire)).unwrap();
        assert_eq!((sent.width(), sent.height()), (7, 8192));
    }

    #[test]
    fn fifteen_repeated_picture_parts_use_the_lower_ceiling() {
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgb8(8, 5000)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let (mut endpoint, mut called) =
            deepseek_picture(bytes.clone(), kernel::ImageType::Png, 8, 5000);
        let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = reads.clone();
        endpoint.redemption = Redemption::new(
            resolver(),
            std::sync::Arc::new(move |_| {
                counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(bytes.clone())
            }),
        );
        let content = &mut called.chat.messages.to_mut()[0].content;
        let picture = content[0].clone();
        content.resize(14, picture.clone());
        let wire = endpoint.wire_request(&called).unwrap();
        assert_eq!(
            image::load_from_memory(&wire_picture(&wire))
                .unwrap()
                .height(),
            5000
        );
        called.chat.messages.to_mut()[0].content.push(picture);
        let wire = endpoint.wire_request(&called).unwrap();
        assert_eq!(wire["messages"][0]["content"].as_array().unwrap().len(), 15);
        assert_eq!(
            image::load_from_memory(&wire_picture(&wire))
                .unwrap()
                .height(),
            4096
        );
        assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn a_deepseek_picture_within_the_side_ceiling_keeps_its_original_bytes() {
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgb8(8, 8)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        let (endpoint, called) =
            deepseek_picture(bytes.clone(), kernel::ImageType::Jpeg, 9000, 9000);

        let wire = endpoint.wire_request(&called).unwrap();
        assert_eq!(wire_picture(&wire), bytes);
    }

    #[test]
    fn an_animated_picture_over_the_side_ceiling_is_not_silently_flattened() {
        let mut bytes = Vec::new();
        image::codecs::gif::GifEncoder::new(&mut bytes)
            .encode_frames([
                image::Frame::new(image::RgbaImage::new(4, 16384)),
                image::Frame::new(image::RgbaImage::new(4, 16384)),
            ])
            .unwrap();
        let (endpoint, called) = deepseek_picture(bytes, kernel::ImageType::Gif, 4, 16384);

        let refused = endpoint.wire_request(&called);
        assert!(
            refused.is_err(),
            "an animated picture crossed without a refusal"
        );
        let err = refused.unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(err.recovery().contains("static frame"), "{err}");
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

        assert!(seen[0].contains("x-api-key: sk-test-0123456789"));
        assert!(seen[0].contains("anthropic-version: 2023-06-01"));

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
    fn the_fifth_distinct_picture_in_one_request_is_refused_before_any_byte_is_sent() {
        let endpoint =
            Endpoint::new(config("http://127.0.0.1:1/v1/messages"), redemption()).unwrap();
        let err = endpoint.wire_request(&seeing(5)).unwrap_err();

        assert_eq!(err, IMAGES_PER_TURN.admit(5).unwrap_err());

        assert!(endpoint.wire_request(&seeing(4)).is_ok());
    }

    #[test]
    fn repeated_references_resolve_once_without_changing_their_wire_positions() {
        let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = std::sync::Arc::clone(&reads);
        let endpoint = Endpoint::new(
            config("http://127.0.0.1:1/v1/messages"),
            Redemption::new(
                resolver(),
                std::sync::Arc::new(move |_at: &kernel::Locator| {
                    counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(b"the-pixels".to_vec())
                }),
            ),
        )
        .unwrap();
        let mut repeated = seeing(1);
        let picture = repeated.chat.messages.to_mut()[0].content[0].clone();
        for _ in 1..5 {
            repeated.chat.messages.to_mut()[0]
                .content
                .push(picture.clone());
        }

        let wire = endpoint.wire_request(&repeated).unwrap();

        assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(wire.to_string().matches("\"type\":\"image\"").count(), 5);
    }

    #[test]
    fn a_picture_larger_than_the_ceiling_is_refused_with_the_ceiling_named() {
        let endpoint = Endpoint::new(
            config("http://127.0.0.1:1/v1/messages"),
            Redemption::new(
                resolver(),
                std::sync::Arc::new(|_at: &kernel::Locator| Ok(vec![0u8; 5_000_000])),
            ),
        )
        .unwrap();
        let request = seeing(1);
        let at = pictures_in(&request.chat)
            .first()
            .map(|picture| picture.locator.clone())
            .expect("the fixture puts one picture in the chat");
        let err = endpoint.wire_request(&request).unwrap_err();
        assert_eq!(err, IMAGE_MAX_BYTES.admit(&at, 5_000_000).unwrap_err());
    }

    #[test]
    fn typed_generation_is_numeric_and_a_raw_override_still_wins() {
        let mut configured = config("http://127.0.0.1:1/v1/messages");
        configured
            .overrides
            .push(("/temperature".to_owned(), serde_json::json!(0.8)));
        let endpoint = Endpoint::new(configured, redemption()).unwrap();
        let mut called = request();
        called.chat.generation = kernel::GenerationSettings {
            temperature: Some("0.2".to_owned()),
            top_p: Some("0.9".to_owned()),
            top_k: Some("40".to_owned()),
        };
        let wire = endpoint.wire_request(&called).unwrap();
        assert_eq!(wire["temperature"], serde_json::json!(0.8));
        assert_eq!(wire["top_p"], serde_json::json!(0.9));
        assert_eq!(wire["top_k"], serde_json::json!(40));
        called.chat.generation.temperature = Some("not-a-number".to_owned());
        assert_eq!(
            *endpoint.wire_request(&called).unwrap_err().code(),
            AxCode::ConfigInvalid
        );
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
