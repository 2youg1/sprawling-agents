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
    use super::super::config::{config, request, resolver};
    use super::*;
    use kernel::BuildingPolicy;
    #[test]
    fn a_confidential_building_never_reaches_a_remote_provider() {
        // No listener at all: if the refusal were routing-level rather
        // than here, this would fail as a transport error instead.
        let mut endpoint =
            Endpoint::new(config("http://127.0.0.1:1/v1/messages"), resolver()).unwrap();
        let mut confidential = request();
        confidential.policy = BuildingPolicy::new(true);
        let err = endpoint.call(&confidential).unwrap_err();
        assert_eq!(err.code(), &AxCode::GateDenied);
        assert!(err.recovery().contains("local model"));
    }
}
