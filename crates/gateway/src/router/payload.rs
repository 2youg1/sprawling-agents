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

use kernel::{
    AxCode, AxError, Ceiling, DialectKind, ModelTag, Payload, Proxying, SecretRef, UsdMicros,
};
use serde_json::{Map, Value};

use crate::endpoint::AuthSpec;
use crate::fallback::Fallback;
use crate::market::{InputKinds, ModelEntry};

use super::attached::AttachedEndpoint;
use super::book::Choice;
use super::tuning::EndpointTuning;

/// The registration's `tuning` object, written only when the person
/// settled something. An endpoint they left alone writes no key, so a
/// record says "nothing was settled" by staying silent about it.
fn tuning_value(tuning: &EndpointTuning) -> Result<Option<Value>, AxError> {
    let mut map = Map::new();
    if let Some(label) = &tuning.label {
        map.insert("label".to_owned(), Value::String(label.clone()));
    }
    for (key, figure) in [
        ("timeout_ms", tuning.timeout_ms),
        ("stream_deadline_ms", tuning.stream_deadline_ms),
    ] {
        if let Some(ms) = figure {
            map.insert(key.to_owned(), Value::Number(ms.into()));
        }
    }
    if let Some(retries) = tuning.request_max_retries {
        map.insert(
            "request_max_retries".to_owned(),
            Value::Number(retries.into()),
        );
    }
    // The default stays silent, so a record says "nothing was settled"
    // the same way it does for every other figure here.
    if tuning.proxying != Proxying::default() {
        let spelled = serde_json::to_value(tuning.proxying).map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode proxying", err.to_string())
        })?;
        map.insert("proxying".to_owned(), spelled);
    }
    for (key, rows) in [
        ("extra_headers", &tuning.extra_headers),
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
            _ => None,
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
fn read_tuning(payload: &Payload) -> EndpointTuning {
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
        request_max_retries: figure("request_max_retries")
            .and_then(|held| u32::try_from(held).ok()),
        stream_deadline_ms: figure("stream_deadline_ms"),
        extra_headers: pairs(tuning, "extra_headers"),
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
            AxError::failure(AxCode::InvalidArgs, "encode dialect", err.to_string())
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
                .map(|id| Value::String(id.clone()))
                .collect(),
        ),
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
        _ => None,
    }
}

/// The one place a `model_selected` payload is formed.
///
/// # Errors
/// Propagates payload construction failure.
pub fn selected_payload(
    tag: ModelTag,
    endpoint: &str,
    entry: &ModelEntry,
    fallback: &Fallback,
) -> Result<Payload, AxError> {
    let mut map = Map::new();
    map.insert("tag".to_owned(), Value::String(tag.as_str().to_owned()));
    map.insert("endpoint".to_owned(), Value::String(endpoint.to_owned()));
    map.insert("model".to_owned(), Value::String(entry.id.clone()));
    // Written only by the retreating arm, so `None` is spelled by the
    // absence of the keys — which is also how every record written
    // before this card spells it.
    if let (Some(name), Some(id)) = (fallback.endpoint(), fallback.model()) {
        map.insert(
            "fallback_endpoint".to_owned(),
            Value::String(name.to_owned()),
        );
        map.insert("fallback_model".to_owned(), Value::String(id.to_owned()));
    }
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
    map.insert(
        "input".to_owned(),
        serde_json::to_value(entry.input).map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode input kinds", err.to_string())
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

pub(crate) fn invalid(subject: impl Into<String>) -> AxError {
    AxError::failure(AxCode::WireMismatch, "read an endpoint record", subject)
        .with_recovery("replay with the build that wrote this record")
}

pub(crate) fn text(payload: &Payload, key: &str) -> Result<String, AxError> {
    payload
        .as_map()
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("{key} is missing or not a string")))
}

/// A registered ceiling, or `None` when this record states none.
///
/// Tolerant in both directions a record can spell absence: a key that is
/// missing or null, and the zero that a build writing `u64` wrote when
/// no catalogue row knew the model. Zero read back as a ceiling is a
/// request the provider answers with nothing, so it reads as unknown.
pub(crate) fn ceiling(payload: &Payload, key: &str) -> Result<Option<Ceiling>, AxError> {
    match payload.as_map().get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Ceiling::new)
            .ok_or_else(|| invalid(format!("{key} is not a count"))),
    }
}

pub(crate) fn count(payload: &Payload, key: &str) -> Result<u64, AxError> {
    payload
        .as_map()
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid(format!("{key} is missing or not a count")))
}

pub(crate) fn read_attached(payload: &Payload) -> Result<AttachedEndpoint, AxError> {
    let dialect_value = payload
        .as_map()
        .get("dialect")
        .ok_or_else(|| invalid("dialect is missing"))?;
    let dialect: DialectKind = serde_json::from_value(dialect_value.clone())
        .map_err(|err| invalid(format!("dialect: {err}")))?;
    let auth = match payload.as_map().get("auth").and_then(Value::as_str) {
        None => AuthSpec::None,
        Some(raw) => {
            let reference = SecretRef::parse(raw)?;
            match payload.as_map().get("auth_header").and_then(Value::as_str) {
                Some(header) => AuthSpec::Header {
                    name: header.to_owned(),
                    value: reference,
                },
                None => AuthSpec::Bearer(reference),
            }
        }
    };
    let models = payload
        .as_map()
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("models is missing or not an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("a model id is not a string"))
        })
        .collect::<Result<Vec<String>, AxError>>()?;
    Ok(AttachedEndpoint {
        name: text(payload, "name")?,
        base_url: text(payload, "base_url")?,
        dialect,
        auth,
        models,
        // Absent means true: every record written before this key
        // existed came from a probe that succeeded, so an old ledger
        // replays into the same book it always did.
        probed: payload
            .as_map()
            .get("probed")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        tuning: read_tuning(payload),
    })
}

pub(crate) fn read_choice(payload: &Payload) -> Result<(ModelTag, Choice), AxError> {
    let raw = text(payload, "tag")?;
    let tag = ModelTag::ALL
        .into_iter()
        .find(|candidate| candidate.as_str() == raw)
        .ok_or_else(|| invalid(format!("{raw} is not a tag this build knows")))?;
    // Tolerant on purpose: every `model_selected` written before this
    // key existed replays as text-only rather than as a broken record.
    let input = match payload.as_map().get("input") {
        None => InputKinds::default(),
        Some(value) => {
            serde_json::from_value(value.clone()).map_err(|err| invalid(format!("input: {err}")))?
        }
    };
    let entry = ModelEntry {
        id: text(payload, "model")?,
        context_tokens: count(payload, "context_tokens")?,
        max_output_tokens: ceiling(payload, "max_output_tokens")?,
        input,
        input_price: UsdMicros::new(count(payload, "input_price")?),
        output_price: UsdMicros::new(count(payload, "output_price")?),
        cache_read_price: UsdMicros::new(count(payload, "cache_read_price")?),
        cache_write_price: UsdMicros::new(count(payload, "cache_write_price")?),
    };
    // Both keys or neither: a half-written retreat names a model with
    // no endpoint to reach it at, and guessing the missing half is the
    // silent model switch this whole value exists to forbid.
    let fallback = match (
        payload
            .as_map()
            .get("fallback_endpoint")
            .and_then(Value::as_str),
        payload
            .as_map()
            .get("fallback_model")
            .and_then(Value::as_str),
    ) {
        (Some(name), Some(id)) => Fallback::then(name, id)?,
        (None, None) => Fallback::None,
        _ => {
            return Err(invalid(
                "a fallback names only half of an endpoint and a model",
            ));
        }
    };
    Ok((
        tag,
        Choice {
            endpoint: text(payload, "endpoint")?,
            entry,
            fallback,
        },
    ))
}
