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

use super::header::HeaderValue;
use super::redemption::Redemption;
use crate::market::ModelEntry;

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
    /// Headers every request adds. A value is a literal or a vault
    /// reference, and which one it is decides whether it is redeemed
    /// in the slot before the wire.
    pub extra_headers: Vec<(String, HeaderValue)>,
    /// Field-by-field request overrides: JSON pointer to value, applied
    /// last, later entries win. Missing object paths are created.
    pub overrides: Vec<(String, Value)>,
    /// How long one settled call may take in total: the whole body is
    /// read under this bound, because a settled call has nothing to
    /// report until it has all of it.
    pub timeout_ms: u64,
    /// How long a *streamed* request may go without a byte arriving.
    /// A model that is still writing is not a model that has stopped
    /// answering, so a stream is bounded by its silences rather than
    /// by its length; `None` bounds each silence by `timeout_ms`.
    pub stream_idle_timeout_ms: Option<u64>,
    /// Price-sheet row for settlement; `None` settles nothing (billed
    /// stays empty and attribution sees usage only).
    pub pricing: Option<ModelEntry>,
    /// Which of this endpoint's calls go through the machine's proxy.
    /// Resolved before it gets here: the wire spells absence, and the
    /// assembly layer turns absence into the city's own default.
    pub proxying: Proxying,
}

/// One endpoint, ready to call.
///
/// The three fields are reachable inside this module tree and nowhere
/// else: a caller that could take the transport out could also decide
/// for itself what a non-2xx means and whether the provider's own body
/// is quoted back, which would put those two answers under two
/// authorities. What the rest of the crate may do with an endpoint is
/// on `impl Endpoint`.
pub struct Endpoint {
    pub(super) config: EndpointConfig,
    pub(super) client: reqwest::blocking::Client,
    pub(super) redemption: Redemption,
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
                    .with_recovery(
                        "check this endpoint's `base_url` and the proxy settings this \
                         machine exports (`HTTPS_PROXY`, `NO_PROXY`)",
                    )
            })?;
        Ok(Endpoint {
            config,
            client,
            redemption,
        })
    }

    /// The provider-side name of the model this endpoint calls.
    ///
    /// The one thing a caller outside this module tree reads about a
    /// configured endpoint: a name, not a handle. Whoever writes a
    /// body that has to name the model asks for the name and gets
    /// nothing else.
    #[must_use]
    pub(crate) fn model(&self) -> &str {
        &self.config.model
    }
}

fn transport_detail(err: &reqwest::Error) -> String {
    let mut out = err.to_string();
    let mut cause = std::error::Error::source(err);
    while let Some(link) = cause {
        out.push_str(": ");
        out.push_str(&link.to_string());
        cause = link.source();
    }
    out
}

/// What failed about one provider call, said by the call site that saw
/// it rather than guessed later from the text of a message.
///
/// **This enum is the one home of "may this be tried again".** An
/// exchange that never completed carries no answer, so the same request
/// may go out again; an answer the city refuses would be refused
/// identically next time. Before this type every call site built its
/// own error and none opted in, which made the default
/// `Retries::UntilHalted` worth zero retries in practice — one
/// transient disconnect ended a sub-run for good.
#[derive(Debug)]
pub(crate) enum ProviderFailure<'e> {
    /// The exchange never completed: the request did not reach the
    /// provider, or the answer's body stopped before its end, or the
    /// transport's deadline passed.
    Exchange(&'e reqwest::Error),
    /// A streamed body could not be read further off the socket.
    Cut(&'e std::io::Error),
    /// A stream stayed open and silent past the bound set for silence.
    Silence { quiet_ms: u64 },
    /// The provider answered and the status refuses the request. The
    /// provider's own body is never quoted back.
    Refused {
        url: &'e str,
        status: reqwest::StatusCode,
    },
    /// The provider answered 2xx and the body is not a shape this city
    /// can read.
    Unreadable(String),
    /// The request could not even be built — a URL, a header or a body
    /// this side wrote wrong. Deterministic: the identical build fails
    /// identically, which is why the one failure here that never
    /// reached a provider is classed with the refusals.
    Unbuilt(&'e reqwest::Error),
}

impl ProviderFailure<'_> {
    fn subject(&self) -> String {
        match self {
            ProviderFailure::Exchange(err) | ProviderFailure::Unbuilt(err) => transport_detail(err),
            ProviderFailure::Cut(err) => err.to_string(),
            ProviderFailure::Silence { quiet_ms } => {
                format!("no byte arrived for {quiet_ms} ms")
            }
            ProviderFailure::Refused { url, status } => {
                format!("{url} answered {}", status.as_u16())
            }
            ProviderFailure::Unreadable(detail) => detail.clone(),
        }
    }

    fn retriable(&self) -> bool {
        match self {
            ProviderFailure::Exchange(_)
            | ProviderFailure::Cut(_)
            | ProviderFailure::Silence { .. } => true,
            ProviderFailure::Refused { .. }
            | ProviderFailure::Unreadable(_)
            | ProviderFailure::Unbuilt(_) => false,
        }
    }

    fn recovery(&self) -> &'static str {
        match self {
            ProviderFailure::Exchange(_)
            | ProviderFailure::Cut(_)
            | ProviderFailure::Silence { .. } => {
                "the watchdog decides retry or failover; admission widens the interval"
            }
            ProviderFailure::Refused { .. } | ProviderFailure::Unreadable(_) => {
                "the provider answered, and it would answer the same way again: check this \
                 endpoint's model name, credential and dialect, then dispatch again"
            }
            ProviderFailure::Unbuilt(_) => {
                "check this endpoint's `base_url` and its extra headers: the request was \
                 refused by this side before it was sent"
            }
        }
    }
}

/// One provider failure as an `E_PROVIDER` error, its retriability and
/// its way out taken from [`ProviderFailure`], never decided here.
pub(crate) fn provider_err(action: &str, failure: &ProviderFailure<'_>) -> AxError {
    let draft = AxError::failure(AxCode::Provider, action, failure.subject());
    let draft = if failure.retriable() {
        draft.retriable()
    } else {
        draft
    };
    draft.with_recovery(failure.recovery())
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
        )
        .with_recovery(
            "give the override a pointer that names a field, for example \
             `/generation_config/temperature`",
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
                Value::Null
                | Value::Bool(_)
                | Value::Number(_)
                | Value::String(_)
                | Value::Array(_) => {
                    return Err(AxError::failure(
                        AxCode::ConfigInvalid,
                        "apply request override",
                        format!("{pointer}: parent is not an object"),
                    )
                    .with_recovery(format!(
                        "shorten `{pointer}` to the object that holds the field, or drop \
                         the override: this request body has a value where the pointer \
                         expects an object"
                    )));
                }
            }
        }
        cursor = match cursor {
            Value::Object(map) => map
                .entry((*segment).to_owned())
                .or_insert_with(|| Value::Object(serde_json::Map::new())),
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Array(_) => {
                return Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "apply request override",
                    format!("{pointer}: crossed a non-object"),
                )
                .with_recovery(format!(
                    "remove the segment of `{pointer}` that lands on a value rather than \
                     an object; an override only reaches fields of objects"
                )));
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
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
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
        // The deadline passed with no answer at all, so the same
        // request is worth sending again; this is the opt-in the
        // watchdog reads before it spends a retry.
        assert!(err.is_retriable(), "a call that never completed");
    }

    #[test]
    fn what_never_completed_is_asked_again_and_what_was_refused_is_not() {
        let cut = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "the body stopped");
        for exchange in [
            ProviderFailure::Cut(&cut),
            ProviderFailure::Silence { quiet_ms: 5 },
        ] {
            assert!(provider_err("read provider response", &exchange).is_retriable());
        }
        let refused = provider_err(
            "call provider",
            &ProviderFailure::Refused {
                url: "http://house/v1",
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            },
        );
        assert!(!refused.is_retriable(), "it would answer the same way");
        assert!(refused.subject().contains("answered 500"));
        let shape = ProviderFailure::Unreadable("no data array".to_owned());
        assert!(!provider_err("read the model list", &shape).is_retriable());
    }
}
