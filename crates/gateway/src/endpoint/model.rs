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

//! The Model face: calls the router can hold.

use kernel::{AxCode, AxError, Model, ModelRequest, ModelReturn, UsdMicros};
use serde_json::Value;

use crate::cost;
use crate::dialect::response_from_wire;
use crate::market::InputKinds;

use super::config::Endpoint;
use super::failure::{ProviderFailure, provider_err};
impl Model for Endpoint {
    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.call_speculating(req, onto, &mut |_| {})
    }

    fn call_speculating(
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

    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        // A confidential building's bytes do not leave the machine, and
        // this type is the way off it. The refusal is here rather than
        // only at the routing layer because a backstop that lives where
        // the leak would happen survives a routing mistake.
        if req.policy.confidential {
            return Err(self.confidential_refusal());
        }
        self.refuse_pictures_a_blind_model_cannot_read(req)?;
        let wire = self.wire_request(req)?;
        let request = self.authorize(
            self.client
                .post(&self.config.base_url)
                .header("content-type", "application/json"),
        )?;
        let response = request
            .json(&wire)
            .send()
            .map_err(|err| provider_err("call provider", &ProviderFailure::Exchange(&err)))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_err(
                "call provider",
                &ProviderFailure::Refused {
                    url: &self.config.base_url,
                    status,
                },
            ));
        }
        // A cut stream surfaces here as a body read error — no partial
        // ModelReturn is ever fabricated.
        let body: Value = response.json().map_err(|err| {
            provider_err("read provider response", &ProviderFailure::Exchange(&err))
        })?;
        self.returned(&body)
    }
}

impl Endpoint {
    /// The return a settled wire answer makes, billed when this endpoint
    /// knows its price. Both doors end here, so a streamed call and a
    /// blocking one given the same answer return the same value.
    pub(super) fn returned(&self, settled: &Value) -> Result<ModelReturn, AxError> {
        let resp = response_from_wire(self.config.dialect, settled)?;
        let billed: Option<UsdMicros> = match &self.config.pricing {
            Some(entry) => Some(cost::settle(&resp.usage, None, entry)?.billed),
            None => None,
        };
        ModelReturn::from_response(resp, billed)
    }

    /// What the chosen model accepts, answered before the request is
    /// built.
    ///
    /// The catalogue row this endpoint was configured with is where the
    /// choice is known, so this is where a picture meets it. Dropping
    /// the picture instead would send a conversation that refers to
    /// something the model was never given.
    fn refuse_pictures_a_blind_model_cannot_read(&self, req: &ModelRequest) -> Result<(), AxError> {
        let blind = self
            .config
            .pricing
            .as_ref()
            .is_some_and(|entry| entry.input == InputKinds::Text);
        if !blind || super::call::pictures_in(&req.chat).is_empty() {
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

    /// The one sentence both doors say when a confidential building asks
    /// to leave this machine. Written once because two copies of a
    /// security refusal are two chances for one of them to soften.
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
        // No listener: a refusal that came from the far side would be a
        // transport error, so reaching E_INVALID_ARGS proves the check
        // ran before the request left.
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
        seeing.chat.messages.push(kernel::ChatMessage {
            cache: false,
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
        // No listener at all: if the refusal were routing-level rather
        // than here, this would fail as a transport error instead.
        let mut endpoint =
            Endpoint::new(config("http://127.0.0.1:1/v1/messages"), redemption()).unwrap();
        let mut confidential = request();
        confidential.policy = BuildingPolicy::new(true);
        let err = endpoint.call(&confidential).unwrap_err();
        assert_eq!(err.code(), &AxCode::GateDenied);
        assert!(err.recovery().contains("local model"));
    }

    /// **A provider that ignores `stream: true` still answers.** Many
    /// OpenAI-compatible servers, local ones most of all, reply to a
    /// stream request with one `application/json` body. Read as frames
    /// it holds none, so the call failed as a cut stream although the
    /// whole answer had arrived, and the run backed off and asked again.
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

    /// **A tool call reaches the caller while the model is still
    /// writing.** The provider sends a complete `read` call, then waits
    /// to hear that the caller has it before it writes the text that
    /// follows; a door that handed the call over only after the answer
    /// settled would leave the server waiting, and it answers `false`.
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
