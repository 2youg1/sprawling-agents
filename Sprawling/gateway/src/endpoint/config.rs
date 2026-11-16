// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, DialectKind, Proxying, SecretRef};
use serde_json::Value;

use super::header::HeaderValue;
use super::redemption::Redemption;
use crate::market::ModelEntry;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AuthSpec {
    Bearer(SecretRef),
    Header { name: String, value: SecretRef },
    None,
}

#[derive(Debug, Clone)]
pub struct EndpointConfig {

    pub base_url: String,
    pub dialect: DialectKind,

    pub model: String,
    pub auth: AuthSpec,

    pub extra_headers: Vec<(String, HeaderValue)>,

    pub overrides: Vec<(String, Value)>,

    pub timeout_ms: u64,

    pub stream_idle_timeout_ms: Option<u64>,

    pub pricing: Option<ModelEntry>,

    pub proxying: Proxying,
}

pub struct Endpoint {
    pub(super) config: EndpointConfig,
    pub(super) client: reqwest::blocking::Client,
    pub(super) redemption: Redemption,

    pub(super) thinking: crate::provider::thinking::ThinkingOffer,
}

impl Endpoint {

    pub fn new(config: EndpointConfig, redemption: Redemption) -> Result<Endpoint, AxError> {
        Endpoint::over(&super::transport::Transport::default(), config, redemption)
    }

    #[must_use]
    pub(crate) fn model(&self) -> &str {
        &self.config.model
    }
}

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
    use kernel::AxCode;
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

        assert!(
            err.subject().to_ascii_lowercase().contains("timed out"),
            "a provider that says nothing must end as a timeout: {}",
            err.subject()
        );

        assert_eq!(
            err.retry(),
            kernel::Retry::Unknown,
            "a call that never completed"
        );
    }
}
