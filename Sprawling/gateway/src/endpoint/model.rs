// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, ModelRequest, ModelReturn, UsdMicros};
use serde_json::Value;

use crate::cost;
use crate::dialect::response_from_wire;
use crate::market::InputKinds;

use super::config::Endpoint;
use super::failure::{ProviderFailure, provider_err};
impl Endpoint {
    pub(crate) fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.call_speculating(req, onto, &mut |_| {})
    }

    pub(crate) fn call_speculating(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
        early: kernel::EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        if req.policy.confidential {
            return Err(self.confidential_refusal());
        }
        self.refuse_pictures_a_blind_model_cannot_read(req)?;
        self.stream(req, onto, early)
    }

    pub(crate) fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        if req.policy.confidential {
            return Err(self.confidential_refusal());
        }
        self.refuse_pictures_a_blind_model_cannot_read(req)?;
        let wire = self.wire_request(req)?;
        self.post_settled(
            req,
            &wire,
            std::time::Duration::from_millis(self.config.timeout_ms),
        )
    }
}

impl Endpoint {
    pub(super) fn post_settled(
        &self,
        req: &ModelRequest,
        wire: &Value,
        timeout: std::time::Duration,
    ) -> Result<ModelReturn, AxError> {
        let request = self.chat_post(req)?;
        let response = request
            .json(wire)
            .timeout(timeout)
            .send()
            .map_err(|err| provider_err("call provider", &ProviderFailure::Exchange(&err)))?;
        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            return Err(provider_err(
                "call provider",
                &ProviderFailure::refusal(
                    &self.config.base_url,
                    status,
                    &headers,
                    &response.text(),
                ),
            ));
        }

        let body: Value = response.json().map_err(|err| {
            provider_err("read provider response", &ProviderFailure::Exchange(&err))
        })?;
        self.returned(&body)
    }
}

impl Endpoint {
    pub(super) fn returned(&self, settled: &Value) -> Result<ModelReturn, AxError> {
        let resp = response_from_wire(self.config.dialect, settled)?;

        let billed: Option<UsdMicros> = match &self.config.pricing {
            Some(entry) if entry.states_a_price() => {
                Some(cost::settle(&resp.usage, None, entry)?.billed)
            }
            Some(_) | None => None,
        };
        ModelReturn::from_response(resp, billed)
    }

    fn refuse_pictures_a_blind_model_cannot_read(&self, req: &ModelRequest) -> Result<(), AxError> {
        if !self.blind_to_this_request(req) {
            return Ok(());
        }
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "send a picture to this model",
            self.config.model.clone(),
        )
        .with_recovery(
            "this model is registered as taking text only; point the tag at a model \
             whose input kind is text_image, or describe the picture in words",
        ))
    }

    pub(super) fn blind_to_this_request(&self, req: &ModelRequest) -> bool {
        self.config
            .pricing
            .as_ref()
            .is_some_and(|entry| entry.input == InputKinds::Text)
            && !super::call::pictures_in(&req.chat).is_empty()
    }

    fn confidential_refusal(&self) -> AxError {
        AxError::failure(
            AxCode::GateDenied,
            "call a remote provider for a confidential building",
            self.config.base_url.clone(),
        )
        .with_recovery(
            "configure a local model for this building, or drop `confidential = true` \
             from its RULES.toml and record why",
        )
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
    use super::super::redemption::redemption;
    use super::*;
    use kernel::BuildingPolicy;

    #[test]
    fn a_model_that_cannot_see_refuses_the_picture_instead_of_dropping_it() {
        let blind = crate::market::MarketSnapshot::builtin()
            .unwrap()
            .lookup("local")
            .unwrap()
            .clone();
        assert_eq!(blind.input, crate::market::InputKinds::Text);
        let mut endpoint = Endpoint::new(
            super::super::config::EndpointConfig {
                pricing: Some(blind),
                ..config("http://127.0.0.1:1/v1/messages")
            },
            redemption(),
        )
        .unwrap();
        let mut seeing = request();
        seeing.chat.messages.to_mut().push(kernel::ChatMessage {
            role: kernel::Role::User,
            content: vec![kernel::ContentBlock::Image(kernel::ImageRef {
                locator: kernel::Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap(),
                media_type: kernel::ImageType::Png,
                width: 8,
                height: 8,
            })],
        });
        let err = endpoint.call(&seeing).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(
            err.recovery().contains("text_image"),
            "the refusal names the input kind a person has to choose: {}",
            err.recovery()
        );
    }
    #[test]
    fn a_price_row_with_no_figure_leaves_the_call_unbilled() {
        let body = serde_json::json!({
            "content": [{ "type": "text", "text": "done" }],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 900, "output_tokens": 100 },
        })
        .to_string();
        let (url, server) = fake_provider(vec![(200, body)], false);
        let free = crate::market::MarketSnapshot::builtin()
            .unwrap()
            .lookup("local")
            .unwrap()
            .clone();
        let mut endpoint = Endpoint::new(
            super::super::config::EndpointConfig {
                pricing: Some(free),
                ..config(&url)
            },
            redemption(),
        )
        .unwrap();
        let ret = endpoint.call(&request()).unwrap();
        server.join().unwrap();
        assert_eq!(ret.billed_usd_micros, None, "nobody priced this call");
    }
    #[test]
    fn a_confidential_building_never_reaches_a_remote_provider() {
        let mut endpoint =
            Endpoint::new(config("http://127.0.0.1:1/v1/messages"), redemption()).unwrap();
        let mut confidential = request();
        confidential.policy = BuildingPolicy::new(true);
        let err = endpoint.call(&confidential).unwrap_err();
        assert_eq!(err.code(), &AxCode::GateDenied);
        assert!(err.recovery().contains("local model"));
    }

    #[test]
    fn a_whole_body_answering_a_stream_request_is_the_answer_a_call_returns() {
        let body = serde_json::json!({
            "content": [ { "type": "text", "text": "whole" } ],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 1, "output_tokens": 1 },
        })
        .to_string();
        let (url, server) = fake_provider(vec![(200, body.clone()), (200, body)], false);
        let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
        let called = endpoint.call(&request()).unwrap();
        let mut onto = |_held: &kernel::Increment| {};
        let streamed = endpoint.call_streaming(&request(), &mut onto).unwrap();
        assert_eq!(streamed, called);
        assert!(server.join().unwrap()[1].contains("\"stream\":true"));
    }

    #[test]
    fn a_tool_call_is_handed_over_before_the_answer_settles() {
        let frame = |value: serde_json::Value| value.to_string();
        let opening = vec![
            frame(serde_json::json!({"type": "message_start",
                                      "message": {"usage": {"input_tokens": 3}}})),
            frame(
                serde_json::json!({"type": "content_block_start", "index": 0,
                "content_block": {"type": "tool_use", "id": "t1", "name": "read", "input": {}}}),
            ),
            frame(
                serde_json::json!({"type": "content_block_delta", "index": 0,
                "delta": {"type": "input_json_delta", "partial_json": "{\"path\":\"a.md\"}"}}),
            ),
            frame(serde_json::json!({"type": "content_block_stop", "index": 0})),
        ];
        let closing = vec![
            frame(
                serde_json::json!({"type": "content_block_start", "index": 1,
                "content_block": {"type": "text", "text": ""}}),
            ),
            frame(
                serde_json::json!({"type": "content_block_delta", "index": 1,
                "delta": {"type": "text_delta", "text": "reading"}}),
            ),
            frame(serde_json::json!({"type": "content_block_stop", "index": 1})),
            frame(serde_json::json!({"type": "message_delta",
                "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 2}})),
        ];
        let (holding, waiting) = std::sync::mpsc::channel();
        let (url, server) = super::super::fakes::fake_stream_provider(opening, waiting, closing);
        let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();

        let mut early = Vec::new();
        let ret = endpoint
            .call_speculating(&request(), &mut |_: &kernel::Increment| {}, &mut |call| {
                early.push(call.clone());
                holding.send(()).unwrap();
            })
            .unwrap();

        assert!(
            server.join().unwrap(),
            "the call was handed over only after the provider finished writing"
        );
        assert_eq!(early, ret.calls);
    }
}
