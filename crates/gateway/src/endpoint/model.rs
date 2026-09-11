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

use super::config::{Endpoint, provider_err, transport_detail};
impl Model for Endpoint {
    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        if req.policy.confidential {
            return Err(self.confidential_refusal());
        }
        self.refuse_pictures_a_blind_model_cannot_read(req)?;
        self.stream(req, onto)
    }

    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        // A confidential building's bytes do not leave the machine, and
        // this type is the way off it. The refusal is here rather than
        // only at the routing layer because a backstop that lives where
        // the leak would happen survives a routing mistake (P1.08).
        if req.policy.confidential {
            return Err(self.confidential_refusal());
        }
        self.refuse_pictures_a_blind_model_cannot_read(req)?;
        let wire = self.wire_request(req)?;
        let mut request = self
            .client
            .post(&self.config.base_url)
            .header("content-type", "application/json");
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
        // A cut stream surfaces here as a body read error — no partial
        // ModelReturn is ever fabricated.
        let body: Value = response
            .json()
            .map_err(|err| provider_err("read provider response", transport_detail(&err)))?;
        let resp = response_from_wire(self.config.dialect, &body)?;
        let billed: Option<UsdMicros> = match &self.config.pricing {
            Some(entry) => Some(cost::settle(&resp.usage, None, entry)?.billed),
            None => None,
        };
        ModelReturn::from_response(resp, billed)
    }
}

impl Endpoint {
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
            "configure a local model for this building, or drop `confidential: true` \
             from its BUILDING.md and record why",
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
    use super::super::fakes::{config, request};
    use super::super::redemption::redemption;
    use super::*;
    use kernel::BuildingPolicy;

    #[test]
    fn a_model_that_cannot_see_refuses_the_picture_instead_of_dropping_it() {
        // No listener: a refusal that came from the far side would be a
        // transport error, so reaching E_INVALID_ARGS proves the check
        // ran before the request left.
        let blind = crate::market::MarketSnapshot::builtin()
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
}
