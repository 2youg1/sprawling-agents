// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Local inference always goes through here. The S3
//! shape: a fixed client for an OpenAI-compatible server on loopback.
//! Not a pass-through: the policy this type owns is exactly what
//! `Endpoint` refuses to own — loopback-only by construction, no
//! credential, dialect pinned. The egress surface does not exist in the
//! type, so confidential buildings can trust it structurally.

use kernel::{AxCode, AxError, DialectKind, Model, ModelRequest, ModelReturn, SecretRef};

use crate::endpoint::{AuthSpec, Endpoint, EndpointConfig, Redemption};
use crate::market::ModelEntry;

#[derive(Debug, Clone)]
pub struct NativeConfig {
    /// Full chat endpoint URL; the host must be loopback.
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
    /// Local pricing is normally zero; a metered local pool may price it.
    pub pricing: Option<ModelEntry>,
    /// Carried rather than assumed: the address is on this machine, and
    /// the city takes such a call off the proxy by default — but a
    /// person who settled otherwise for this endpoint settled it for
    /// every adapter that endpoint is reached through.
    pub proxying: kernel::Proxying,
}

pub struct Native {
    inner: Endpoint,
}

impl Native {
    /// Fail-closed: a non-loopback URL is a config error, not a warning.
    pub fn new(config: NativeConfig) -> Result<Native, AxError> {
        if !crate::is_local(&config.base_url) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "configure native model",
                format!("{} is not a loopback address", config.base_url),
            )
            .with_recovery("native serves local inference only; use an endpoint for remote"));
        }
        let inner = Endpoint::new(
            EndpointConfig {
                base_url: config.base_url,
                dialect: DialectKind::OpenAi,
                model: config.model,
                auth: AuthSpec::None,
                extra_headers: Vec::new(),
                overrides: Vec::new(),
                timeout_ms: config.timeout_ms,
                stream_deadline_ms: None,
                pricing: config.pricing,
                proxying: config.proxying,
            },
            // Local inference authenticates with nothing and reads no
            // content store: both redemptions refuse by construction.
            Redemption::without_images(Box::new(|_reference: &SecretRef| {
                Err(AxError::failure(
                    AxCode::CredentialMissing,
                    "resolve credential",
                    "native never authenticates",
                ))
            })),
        )?;
        Ok(Native { inner })
    }
}

impl Model for Native {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.inner.call(req)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn non_loopback_urls_cannot_be_spelled() {
        for bad in [
            "https://api.example.com/v1/chat/completions",
            "http://192.168.1.5:8080/v1/chat/completions",
            "not a url",
        ] {
            let err = match Native::new(NativeConfig {
                base_url: bad.to_owned(),
                model: "local".to_owned(),
                timeout_ms: 1_000,
                pricing: None,
                proxying: kernel::Proxying::default(),
            }) {
                Err(err) => err,
                Ok(_) => panic!("{bad} must be refused"),
            };
            assert_eq!(*err.code(), AxCode::ConfigInvalid, "{bad}");
        }
    }

    #[test]
    fn loopback_spellings_construct() {
        for good in [
            "http://127.0.0.1:8080/v1/chat/completions",
            "http://localhost:11434/v1/chat/completions",
            "http://[::1]:8080/v1/chat/completions",
        ] {
            assert!(
                Native::new(NativeConfig {
                    base_url: good.to_owned(),
                    model: "local".to_owned(),
                    timeout_ms: 1_000,
                    pricing: None,
                    proxying: kernel::Proxying::default(),
                })
                .is_ok(),
                "{good}"
            );
        }
    }
}
