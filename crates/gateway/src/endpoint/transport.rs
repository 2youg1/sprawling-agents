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
pub(crate) struct Transport {
    slot: Arc<OnceLock<reqwest::blocking::Client>>,
    /// The step a test takes after the client is configured and before
    /// it is built (gateway-SPEC.md section 8-32).
    #[cfg(test)]
    detour: Option<Detour>,
}

impl Transport {
    /// The shared client, built from `config` if this is the first call.
    ///
    /// Two callers racing on an empty slot each build one; the slot keeps
    /// the first and the other is dropped with its thread.
    fn client(&self, config: &EndpointConfig) -> Result<reqwest::blocking::Client, AxError> {
        if let Some(client) = self.slot.get() {
            return Ok(client.clone());
        }
        let built = self.build(config)?;
        Ok(self.slot.get_or_init(|| built).clone())
    }

    /// A transport whose client takes `step` after the endpoint's own
    /// configuration: what a test uses to reach a stand-in under a
    /// preset host's name (`reach::resolve`).
    #[cfg(test)]
    pub(crate) fn detoured(
        step: impl Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder
        + Send
        + Sync
        + 'static,
    ) -> Transport {
        Transport {
            slot: Arc::default(),
            detour: Some(Detour(Arc::new(step))),
        }
    }

    fn build(&self, config: &EndpointConfig) -> Result<reqwest::blocking::Client, AxError> {
        let builder = crate::client_for(config.proxying, &config.base_url)
            .timeout(Duration::from_millis(config.timeout_ms));
        #[cfg(test)]
        let builder = match &self.detour {
            Some(Detour(step)) => step(builder),
            None => builder,
        };
        builder.build().map_err(|err| {
            AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
                .with_recovery(
                    "check this endpoint's `base_url` and the proxy settings this \
                     machine exports (`HTTPS_PROXY`, `NO_PROXY`)",
                )
        })
    }
}

/// A step after a client's configuration, held so a transport stays
/// `Clone` and `Debug`.
#[cfg(test)]
#[derive(Clone)]
struct Detour(
    Arc<dyn Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder + Send + Sync>,
);

#[cfg(test)]
impl std::fmt::Debug for Detour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Detour")
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
