// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The book of attached endpoints and the models chosen from them
//! (shape 7): a value rebuilt from the ledger, never a store. Deleting
//! it and replaying the city produces the same book.
//!
//! Two questions live here and nowhere else. *What did the person
//! attach* — a base URL, a dialect, a credential reference, and the
//! model ids the endpoint reported. *Which model answers for a tag* —
//! the choice a request needs, with the facts no probe returns.
//!
//! Duty pools and multi-axis grading are deliberately absent: with one
//! agent per run there is no consumer for a pool, and a pool without a
//! consumer is an authority nobody reads.

//! Payload faces: references on the wire, never credentials.

use kernel::{AxCode, AxError, ModelTag, Payload, Proxying, SecretRef};
use serde_json::{Map, Value};

use crate::endpoint::{AuthSpec, HeaderValue};
use crate::market::ModelEntry;

use super::attached::AttachedEndpoint;
use super::tuning::EndpointTuning;

/// The registration's `tuning` object, written only when the person
/// settled something. An endpoint they left alone writes no key, so a
/// record says "nothing was settled" by staying silent about it.
mod reading;

pub(crate) use reading::{read_attached, read_choice, text};

fn tuning_value(tuning: &EndpointTuning) -> Result<Option<Value>, AxError> {
    let mut map = Map::new();
    if let Some(label) = &tuning.label {
        map.insert("label".to_owned(), Value::String(label.clone()));
    }
    for (key, figure) in [
        ("timeout_ms", tuning.timeout_ms),
        ("stream_idle_timeout_ms", tuning.stream_idle_timeout_ms),
    ] {
        if let Some(ms) = figure {
            map.insert(key.to_owned(), Value::Number(ms.into()));
        }
    }
    if let Some(retries) = tuning.request_max_retries.stated() {
        map.insert(
            "request_max_retries".to_owned(),
            Value::Number(retries.into()),
        );
    }
    // The default stays silent, so a record says "nothing was settled"
    // the same way it does for every other figure here.
    if tuning.proxying != Proxying::default() {
        let spelled = serde_json::to_value(tuning.proxying).map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode proxying", err.to_string()).with_recovery(
                "report this against gateway::router::payload: `Proxying` is a plain \
                     enum and JSON refuses none of its spellings",
            )
        })?;
        map.insert("proxying".to_owned(), spelled);
    }
    // A header's value is written as the person's own spelling, which
    // for a credential is the reference and never the key: that is what
    // keeps `endpoint_attached` exportable.
    let headers: Vec<(String, String)> = tuning
        .extra_headers
        .iter()
        .map(|(name, value)| (name.clone(), value.spelled()))
        .collect();
    for (key, rows) in [
        ("extra_headers", &headers),
        ("overrides", &tuning.overrides),
    ] {
        if !rows.is_empty() {
            map.insert(
                key.to_owned(),
                Value::Array(
                    rows.iter()
                        .map(|(name, value)| {
                            Value::Array(vec![
                                Value::String(name.clone()),
                                Value::String(value.clone()),
                            ])
                        })
                        .collect(),
                ),
            );
        }
    }
    Ok((!map.is_empty()).then_some(Value::Object(map)))
}

/// The pairs a `tuning` object holds under one key, as the record
/// spells them: a list of two-string arrays. A row that is not two
/// strings is left out rather than half read.
fn pairs(tuning: Option<&Value>, key: &str) -> Vec<(String, String)> {
    let Some(Value::Array(rows)) = tuning.and_then(|held| held.get(key)) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| match row {
            Value::Array(cells) => match (cells.first(), cells.get(1)) {
                (Some(Value::String(name)), Some(Value::String(value))) => {
                    Some((name.clone(), value.clone()))
                }
                _ => None,
            },
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Object(_) => None,
        })
        .collect()
}

/// How the person set this endpoint up, as the record kept it.
///
/// Tolerant in one direction only: a key that is missing reads as
/// "nothing was settled", which is what every record written before
/// this object existed means. A key that is present and unreadable is
/// left out rather than guessed at, because a deadline this city
/// invented would be a deadline nobody can explain.
pub(super) fn read_tuning(payload: &Payload) -> EndpointTuning {
    let tuning = payload.as_map().get("tuning");
    let figure = |key: &str| {
        tuning
            .and_then(|held| held.get(key))
            .and_then(Value::as_u64)
    };
    EndpointTuning {
        label: tuning
            .and_then(|held| held.get("label"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        timeout_ms: figure("timeout_ms"),
        request_max_retries: kernel::Retries::of(
            figure("request_max_retries").and_then(|held| u32::try_from(held).ok()),
        ),
        stream_idle_timeout_ms: figure("stream_idle_timeout_ms"),
        extra_headers: read_headers(tuning),
        overrides: pairs(tuning, "overrides"),
        // A record written before this key existed, and a record whose
        // value this build cannot read, both replay as the default:
        // every such record was written by a build that took it.
        proxying: tuning
            .and_then(|held| held.get("proxying"))
            .and_then(|held| serde_json::from_value(held.clone()).ok())
            .unwrap_or_default(),
    }
}
pub fn attached_payload(endpoint: &AttachedEndpoint) -> Result<Payload, AxError> {
    let mut map = Map::new();
    map.insert("name".to_owned(), Value::String(endpoint.name.clone()));
    map.insert(
        "base_url".to_owned(),
        Value::String(endpoint.base_url.clone()),
    );
    map.insert(
        "dialect".to_owned(),
        serde_json::to_value(endpoint.dialect).map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode dialect", err.to_string()).with_recovery(
                "report this against gateway::router::payload: `Dialect` is a plain \
                     enum and JSON refuses none of its spellings",
            )
        })?,
    );
    // The reference, never the credential: this is the byte that makes
    // "plaintext only enters the vault" true of the ledger as well.
    if let Some(reference) = auth_reference(&endpoint.auth) {
        map.insert("auth".to_owned(), Value::String(reference.to_string()));
        if let AuthSpec::Header { name, .. } = &endpoint.auth {
            map.insert("auth_header".to_owned(), Value::String(name.clone()));
        }
    }
    map.insert(
        "models".to_owned(),
        Value::Array(
            endpoint
                .models
                .iter()
                .map(|row| Value::String(row.id.clone()))
                .collect(),
        ),
    );
    map.insert(
        "connection_kind".to_owned(),
        Value::String(endpoint.connection_kind.as_str().to_owned()),
    );
    map.insert("probed".to_owned(), Value::Bool(endpoint.probed));
    if let Some(tuning) = tuning_value(&endpoint.tuning)? {
        map.insert("tuning".to_owned(), tuning);
    }
    Payload::new(map)
}

pub(crate) fn auth_reference(auth: &AuthSpec) -> Option<&SecretRef> {
    match auth {
        AuthSpec::Bearer(reference) => Some(reference),
        AuthSpec::Header { value, .. } => Some(value),
        AuthSpec::None => None,
    }
}

/// The extra headers a record kept, read back as values.
///
/// Entry is where a value that reads as a credential is refused, so a
/// record that holds one was written before that refusal existed.
/// Replay keeps it as the literal it is: the key is already in the
/// ledger, and dropping the header here would change how a person's
/// endpoint is called without telling them.
fn read_headers(tuning: Option<&Value>) -> Vec<(String, HeaderValue)> {
    pairs(tuning, "extra_headers")
        .into_iter()
        .map(|(name, text)| {
            let value = match HeaderValue::parse(&name, &text) {
                Ok(value) => value,
                Err(_) => HeaderValue::Plain(text),
            };
            (name, value)
        })
        .collect()
}

/// The one place a `model_selected` payload is formed.
///
/// `ceiling_from` names which rung of the ladder supplied the output
/// ceiling, when one did: `anthropic.rs` once carried the note that a
/// ceiling invented at the call site truncates runs for a reason that
/// appears nowhere in the account, and this is where that reason
/// appears, in the spelling [`crate::CeilingSource::as_str`] owns.
///
/// # Errors
/// Propagates payload construction failure.
pub fn selected_payload(
    tag: ModelTag,
    endpoint: &str,
    entry: &ModelEntry,
    ceiling_from: Option<crate::CeilingSource>,
) -> Result<Payload, AxError> {
    let mut map = Map::new();
    map.insert("tag".to_owned(), Value::String(tag.as_str().to_owned()));
    map.insert("endpoint".to_owned(), Value::String(endpoint.to_owned()));
    map.insert("model".to_owned(), Value::String(entry.id.clone()));
    map.insert(
        "context_tokens".to_owned(),
        Value::Number(entry.context_tokens.into()),
    );
    map.insert(
        "max_output_tokens".to_owned(),
        match entry.max_output_tokens {
            Some(ceiling) => Value::Number(ceiling.get().into()),
            None => Value::Null,
        },
    );
    if let Some(rung) = ceiling_from {
        map.insert(
            "ceiling_from".to_owned(),
            Value::String(rung.as_str().to_owned()),
        );
    }
    map.insert(
        "input".to_owned(),
        serde_json::to_value(entry.input).map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode input kinds", err.to_string())
                .with_recovery(
                    "report this against gateway::router::payload: `InputKinds` is a plain \
                     enum and JSON refuses none of its spellings",
                )
        })?,
    );
    for (key, price) in [
        ("input_price", entry.input_price),
        ("output_price", entry.output_price),
        ("cache_read_price", entry.cache_read_price),
        ("cache_write_price", entry.cache_write_price),
    ] {
        map.insert(key.to_owned(), Value::Number(price.get().into()));
    }
    Payload::new(map)
}
