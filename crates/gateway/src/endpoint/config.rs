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

//! Endpoint configuration: auth, overrides, construction.

use std::time::Duration;

use kernel::{AxCode, AxError, DialectKind, Proxying, SecretRef};
use serde_json::Value;

use super::redemption::Redemption;
use crate::market::ModelEntry;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum AuthSpec {
    Bearer(SecretRef),
    Header { name: String, value: SecretRef },
    None,
}

/// Everything one endpoint needs, field by field — no provider
/// abstraction layer eats any of it.
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// The full URL of the chat endpoint (e.g. `.../v1/messages`).
    pub base_url: String,
    pub dialect: DialectKind,
    /// Provider-side model name; overrides the duty name from the
    /// canonical request.
    pub model: String,
    pub auth: AuthSpec,
    /// Plaintext headers (never credentials — those go through `auth`).
    pub extra_headers: Vec<(String, String)>,
    /// Field-by-field request overrides: JSON pointer to value, applied
    /// last, later entries win. Missing object paths are created.
    pub overrides: Vec<(String, Value)>,
    pub timeout_ms: u64,
    /// How long a *streamed* request may run, when that is not
    /// `timeout_ms`. A model that is still writing is not a model that
    /// has stopped answering, so a stream is given its own deadline;
    /// `None` holds a stream to the same deadline as a settled call.
    pub stream_deadline_ms: Option<u64>,
    /// Price-sheet row for settlement; `None` settles nothing (billed
    /// stays empty and attribution sees usage only).
    pub pricing: Option<ModelEntry>,
    /// Which of this endpoint's calls go through the machine's proxy.
    /// Resolved before it gets here: the wire spells absence, and the
    /// assembly layer turns absence into the city's own default.
    pub proxying: Proxying,
}

pub struct Endpoint {
    pub(crate) config: EndpointConfig,
    pub(crate) client: reqwest::blocking::Client,
    pub(crate) redemption: Redemption,
}

/// A transport failure as its whole chain states it.
///
/// `reqwest`'s own `Display` says only that sending failed; whether it
/// was a refused connection, an unresolvable name or a passed deadline
/// lives one link further down - and that link is the entire difference
/// between "the provider is down" and "the provider is slow" for the
/// person reading the refusal.
impl Endpoint {
    pub fn new(config: EndpointConfig, redemption: Redemption) -> Result<Endpoint, AxError> {
        let client = crate::client_for(config.proxying, &config.base_url)
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|err| {
                AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
            })?;
        Ok(Endpoint {
            config,
            client,
            redemption,
        })
    }
}

pub(crate) fn transport_detail(err: &reqwest::Error) -> String {
    let mut out = err.to_string();
    let mut cause = std::error::Error::source(err);
    while let Some(link) = cause {
        out.push_str(": ");
        out.push_str(&link.to_string());
        cause = link.source();
    }
    out
}

pub(crate) fn provider_err(action: &str, subject: impl Into<String>) -> AxError {
    AxError::failure(AxCode::Provider, action, subject)
        .with_recovery("the watchdog decides retry or failover; admission widens the interval")
}

/// Applies one JSON-pointer override, creating missing object segments.
pub(crate) fn apply_override(
    root: &mut Value,
    pointer: &str,
    value: &Value,
) -> Result<(), AxError> {
    let path: Vec<&str> = pointer.split('/').filter(|s| !s.is_empty()).collect();
    if path.is_empty() {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "apply request override",
            "empty JSON pointer",
        ));
    }
    let mut cursor = root;
    for (i, segment) in path.iter().enumerate() {
        let last = i == path.len().saturating_sub(1);
        if last {
            match cursor {
                Value::Object(map) => {
                    map.insert((*segment).to_owned(), value.clone());
                    return Ok(());
                }
                _ => {
                    return Err(AxError::failure(
                        AxCode::ConfigInvalid,
                        "apply request override",
                        format!("{pointer}: parent is not an object"),
                    ));
                }
            }
        }
        cursor = match cursor {
            Value::Object(map) => map
                .entry((*segment).to_owned())
                .or_insert_with(|| Value::Object(serde_json::Map::new())),
            _ => {
                return Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "apply request override",
                    format!("{pointer}: crossed a non-object"),
                ));
            }
        };
    }
    Ok(())
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
    use super::super::fakes::{config, fake_provider, request, silent_provider};
    use super::super::redemption::redemption;
    use super::*;
    use kernel::{AxCode, Model};
    #[test]
    fn the_probe_reads_ids_and_carries_the_same_credential_the_call_does() {
        let body = serde_json::json!({
            "data": [
                { "id": "m-large", "object": "model" },
                { "id": "m-small", "object": "model" },
                { "id": "m-large", "object": "model" },
            ],
        })
        .to_string();
        let (url, handle) = fake_provider(vec![(200, body)], false);
        let endpoint = Endpoint::new(config(&url), redemption()).unwrap();
        let served = endpoint.list_models(&url).unwrap();
        assert_eq!(
            served.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            vec!["m-large", "m-small"]
        );
        let seen = handle.join().unwrap();
        assert!(seen[0].starts_with("GET "));
        assert!(seen[0].to_ascii_lowercase().contains("x-api-key: sk-test-"));
    }

    #[test]
    fn a_model_list_without_ids_is_a_provider_error_not_an_empty_city() {
        let (url, handle) = fake_provider(vec![(200, "{\"object\":\"list\"}".to_owned())], false);
        let endpoint = Endpoint::new(config(&url), redemption()).unwrap();
        let err = endpoint.list_models(&url).unwrap_err();
        assert_eq!(err.code(), &AxCode::Provider);
        assert!(err.subject().contains("no data array"));
        let _ = handle.join();
    }

    #[test]
    fn a_provider_that_never_answers_is_given_up_on_at_the_deadline() {
        let url = silent_provider();
        let mut endpoint = Endpoint::new(
            EndpointConfig {
                timeout_ms: 300,
                ..config(&url)
            },
            redemption(),
        )
        .unwrap();
        let err = endpoint.call(&request()).unwrap_err();
        assert_eq!(
            err.code(),
            &AxCode::Provider,
            "silence is a provider failure, and the watchdog's business"
        );
        // The deadline, not the far end, is what ended this call: the
        // server holds the connection open and answers nothing, so a
        // build whose client ignored `timeout` would sit here instead.
        // Asserted on the reason rather than on elapsed time, because
        // this workspace samples no clock (determinism rule two).
        assert!(
            err.subject().to_ascii_lowercase().contains("timed out"),
            "a provider that says nothing must end as a timeout: {}",
            err.subject()
        );
    }
}
