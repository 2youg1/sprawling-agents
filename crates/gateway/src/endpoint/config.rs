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

use kernel::{AxCode, AxError, DialectKind, SecretRef};
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
    /// Price-sheet row for settlement; `None` settles nothing (billed
    /// stays empty and attribution sees usage only).
    pub pricing: Option<ModelEntry>,
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
        let client = reqwest::blocking::Client::builder()
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
use kernel::{B3Hash, BuildingPolicy, ChatRequest, ModelRequest};
#[cfg(test)]
/// A one-shot loopback provider: answers `responses` in order, then
/// closes. `cut_mid_body` truncates the JSON body mid-way.
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn fake_provider(
    responses: Vec<(u16, String)>,
    cut_mid_body: bool,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 65536];
            let mut request = String::new();
            loop {
                let n = stream.read(&mut buf).unwrap();
                request.push_str(&String::from_utf8_lossy(&buf[..n]));
                if let Some(head_end) = request.find("\r\n\r\n") {
                    let head = &request[..head_end];
                    let content_length = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= head_end + 4 + content_length {
                        break;
                    }
                }
            }
            seen.push(request.clone());
            let payload = if cut_mid_body {
                let cut = body.len() / 2;
                let truncated = &body[..cut];
                format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{truncated}",
                    body.len()
                )
            } else {
                format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            stream.write_all(payload.as_bytes()).unwrap();
            drop(stream);
        }
        seen
    });
    (format!("http://{addr}/v1/messages"), handle)
}

/// A provider that accepts the connection and then says nothing.
///
/// This is the shape of a hang, and it is worse than a refusal: a
/// refusal is an answer the run can act on, while silence stops the
/// worker thread that would have acted. The deadline is the only
/// thing standing between the two, so it gets its own test.
/// A provider that accepts the connection and then says nothing.
///
/// This is the shape of a hang, and it is worse than a refusal: a
/// refusal is an answer the run can act on, while silence stops the
/// worker thread that would have acted. The deadline is the only
/// thing standing between the two, so it gets its own test.
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn silent_provider() -> String {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let held = listener.accept();
        // Hold the connection open without answering, long enough
        // for any deadline under test to pass.
        std::thread::sleep(Duration::from_secs(5));
        drop(held);
    });
    format!("http://{addr}/v1/messages")
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
    reason = "test helper"
)]
pub(crate) fn config(url: &str) -> EndpointConfig {
    EndpointConfig {
        base_url: url.to_owned(),
        dialect: DialectKind::Anthropic,
        model: "provider-model".to_owned(),
        auth: AuthSpec::Header {
            name: "x-api-key".to_owned(),
            value: SecretRef::parse("secret:anthropic/api").unwrap(),
        },
        extra_headers: vec![("anthropic-version".to_owned(), "2023-06-01".to_owned())],
        overrides: vec![(
            "/metadata/user_id".to_owned(),
            Value::String("city".to_owned()),
        )],
        timeout_ms: 5_000,
        pricing: Some(
            crate::market::MarketSnapshot::builtin()
                .lookup("claude-sonnet")
                .unwrap()
                .clone(),
        ),
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
    reason = "test helper"
)]
pub(crate) fn request() -> ModelRequest {
    ModelRequest {
        policy: BuildingPolicy::default(),
        segments: [B3Hash::digest(b"seg"); 4],
        chat: ChatRequest::empty("duty", 128),
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
        let ids = endpoint.list_models(&url).unwrap();
        assert_eq!(ids, vec!["m-large".to_owned(), "m-small".to_owned()]);
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
