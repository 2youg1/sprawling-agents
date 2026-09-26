// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One endpoint's HTTP client, and the one place a client is built.
//!
//! Every blocking client starts its own `reqwest-internal-sync-runtime`
//! thread and keeps its own connection pool. A client built per call
//! therefore held one thread per concurrent run and handshook again on
//! every call, so the book keeps one [`Transport`] per endpoint and every
//! call to that endpoint clones its client (gateway-SPEC.md §8-3).

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use kernel::{AxCode, AxError};

use super::config::{Endpoint, EndpointConfig};
use super::redemption::Redemption;

/// One endpoint's client, built the first time a call needs it.
///
/// Clones share the slot, so a copy of the book hands out the same
/// client. Building is deferred because a book is folded on every
/// replay, and an endpoint the city never calls should hold no thread.
#[derive(Debug, Clone, Default)]
pub(crate) struct Transport(Arc<OnceLock<reqwest::blocking::Client>>);

impl Transport {
    /// The shared client, built from `config` if this is the first call.
    ///
    /// Two callers racing on an empty slot each build one; the slot keeps
    /// the first and the other is dropped with its thread.
    fn client(&self, config: &EndpointConfig) -> Result<reqwest::blocking::Client, AxError> {
        if let Some(client) = self.0.get() {
            return Ok(client.clone());
        }
        let built = build(config)?;
        Ok(self.0.get_or_init(|| built).clone())
    }
}

impl Endpoint {
    /// One endpoint, calling through `transport`'s client.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when no client can be built for this
    /// `base_url` and the machine's proxy settings.
    pub(crate) fn over(
        transport: &Transport,
        config: EndpointConfig,
        redemption: Redemption,
    ) -> Result<Endpoint, AxError> {
        Ok(Endpoint {
            client: transport.client(&config)?,
            config,
            redemption,
        })
    }
}

fn build(config: &EndpointConfig) -> Result<reqwest::blocking::Client, AxError> {
    crate::client_for(config.proxying, &config.base_url)
        .timeout(Duration::from_millis(config.timeout_ms))
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
                .with_recovery(
                    "check this endpoint's `base_url` and the proxy settings this \
                     machine exports (`HTTPS_PROXY`, `NO_PROXY`)",
                )
        })
}
