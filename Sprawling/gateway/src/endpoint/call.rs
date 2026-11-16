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
mod tests;
