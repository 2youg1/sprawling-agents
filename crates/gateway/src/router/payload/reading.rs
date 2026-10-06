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

use kernel::event::record::{AttachedTuning, EndpointAttached, ModelSelected};
use kernel::{AxError, DialectKind, ModelTag, Payload, Retries};

use super::super::AttachedEndpoint;
use super::super::book::Choice;
use super::super::tuning::EndpointTuning;
use crate::concurrency::MaxInFlight;
use crate::endpoint::{AuthSpec, HeaderValue, ModelFacts};
use crate::market::ModelEntry;
use crate::provider::registry::ConnectionKind;

pub(crate) fn read_attached(payload: &Payload) -> Result<AttachedEndpoint, AxError> {
    let attached: EndpointAttached = payload.read()?;
    let auth = match (attached.auth, attached.auth_header) {
        (None, _) => AuthSpec::None,
        (Some(value), Some(name)) => AuthSpec::Header { name, value },
        (Some(reference), None) => AuthSpec::Bearer(reference),
    };
    // A line written before this key existed was written by a build
    // that had one registration per format, so the format is what the
    // connection was.
    let connection_kind = match attached.connection_kind {
        Some(word) => ConnectionKind::parse(&word)?,
        None => match attached.dialect {
            DialectKind::Anthropic => ConnectionKind::AnthropicNative,
            DialectKind::OpenAi => ConnectionKind::OpenAiCompat,
            DialectKind::OpenAiResponses => ConnectionKind::Responses,
        },
    };
    Ok(AttachedEndpoint {
        name: attached.name,
        base_url: attached.base_url,
        dialect: attached.dialect,
        connection_kind,
        auth,
        // The line carries ids, because that is all a probe ever wrote
        // into it. What the provider said about each row is read from
        // the market when the row is used, so a replay does not invent
        // facts the history never held.
        models: attached
            .models
            .into_iter()
            .map(|id| ModelFacts {
                id,
                context_tokens: None,
                max_output_tokens: None,
                input_modalities: Vec::new(),
                input_price: None,
                output_price: None,
            })
            .collect(),
        probed: attached.probed,
        tuning: tuning_of(attached.tuning.unwrap_or_default())?,
    })
}

/// How the person set this endpoint up, as the line kept it; what the
/// line left out reads as nothing settled.
fn tuning_of(kept: AttachedTuning) -> Result<EndpointTuning, AxError> {
    super::super::tuning::validate_accounts(kept.accounts.as_deref())?;
    Ok(EndpointTuning {
        accounts: kept.accounts,
        label: kept.label,
        timeout_ms: kept.timeout_ms,
        request_max_retries: Retries::of(kept.request_max_retries),
        stream_idle_timeout_ms: kept.stream_idle_timeout_ms,
        extra_headers: kept
            .extra_headers
            .into_iter()
            .map(|(name, text)| {
                // Entry is where a value that reads as a credential is
                // refused, so a line that holds one was written before
                // that refusal existed. Replay keeps it as the literal
                // it is: the key is already in the ledger, and dropping
                // the header here would change how a person's endpoint
                // is called without telling them.
                let value = match HeaderValue::parse(&name, &text) {
                    Ok(value) => value,
                    Err(_) => HeaderValue::Plain(text),
                };
                (name, value)
            })
            .collect(),
        overrides: kept.overrides,
        proxying: kept.proxying.unwrap_or_default(),
        // A ceiling outside 1 to 256 was never admitted at entry, so a
        // line holding one reads as nothing settled, like every other
        // tuning key this build cannot read.
        max_in_flight: match kept.max_in_flight.map(MaxInFlight::try_from) {
            Some(Ok(ceiling)) => Some(ceiling),
            Some(Err(_)) | None => None,
        },
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
