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

use kernel::event::record::ModelSelected;
use kernel::{AxCode, AxError, DialectKind, ModelTag, Payload, SecretRef};
use serde_json::Value;

use super::super::AttachedEndpoint;
use super::super::book::Choice;
use super::read_tuning;
use crate::endpoint::{AuthSpec, ModelFacts};
use crate::market::ModelEntry;
use crate::provider::registry::ConnectionKind;

fn invalid(subject: impl Into<String>) -> AxError {
    AxError::failure(AxCode::WireMismatch, "read an endpoint record", subject)
        .with_recovery("replay with the build that wrote this record")
}

fn text(payload: &Payload, key: &str) -> Result<String, AxError> {
    payload
        .as_map()
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("{key} is missing or not a string")))
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
            // The record carries ids, because that is all a probe ever
            // wrote into it. What the provider said about each row is
            // read from the market when the row is used, so a replay
            // does not invent facts the history never held.
            value
                .as_str()
                .map(|id| ModelFacts {
                    id: id.to_owned(),
                    context_tokens: None,
                    max_output_tokens: None,
                    input_modalities: Vec::new(),
                    input_price: None,
                    output_price: None,
                })
                .ok_or_else(|| invalid("a model id is not a string"))
        })
        .collect::<Result<Vec<ModelFacts>, AxError>>()?;
    Ok(AttachedEndpoint {
        name: text(payload, "name")?,
        base_url: text(payload, "base_url")?,
        dialect,
        // A record written before this key existed was written by a
        // build that had one registration per format, so the format is
        // what the connection was.
        connection_kind: match payload
            .as_map()
            .get("connection_kind")
            .and_then(Value::as_str)
        {
            Some(word) => ConnectionKind::parse(word)?,
            None => match dialect {
                DialectKind::Anthropic => ConnectionKind::AnthropicNative,
                DialectKind::OpenAi => ConnectionKind::OpenAiCompat,
                DialectKind::OpenAiResponses => ConnectionKind::Responses,
            },
        },
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
    let selected: ModelSelected = payload.read()?;
    let max_output_tokens = selected.ceiling();
    // A record written while this build carried a retreat arm may hold
    // `fallback_endpoint` and `fallback_model`. Nothing ever wrote a
    // value into them and nothing acts on them now, so they are read
    // past in the same way as any other key this book does not own.
    Ok((
        selected.tag,
        Choice {
            endpoint: selected.endpoint,
            entry: ModelEntry {
                id: selected.model,
                context_tokens: selected.context_tokens,
                max_output_tokens,
                input: selected.input,
                input_price: selected.input_price,
                output_price: selected.output_price,
                cache_read_price: selected.cache_read_price,
                cache_write_price: selected.cache_write_price,
            },
        },
    ))
}
