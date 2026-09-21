// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading a record back out of the ledger.
//!
//! The writers in the parent module decide what a record says; these
//! decide what a reader may conclude from one. They are separate
//! because the two directions fail differently: a write fails when this
//! build cannot encode something it holds, and a read fails when the
//! history holds something this build cannot name.

use kernel::{AxCode, AxError, Ceiling, DialectKind, ModelTag, Payload, SecretRef, UsdMicros};
use serde_json::Value;

use super::super::AttachedEndpoint;
use super::super::book::Choice;
use super::read_tuning;
use crate::endpoint::AuthSpec;
use crate::market::{InputKinds, ModelEntry};

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
    // A record written while this build carried a retreat arm may hold
    // `fallback_endpoint` and `fallback_model`. Nothing ever wrote a
    // value into them and nothing acts on them now, so they are read
    // past in the same way as any other key this book does not own.
    Ok((
        tag,
        Choice {
            endpoint: text(payload, "endpoint")?,
            entry,
        },
    ))
}
