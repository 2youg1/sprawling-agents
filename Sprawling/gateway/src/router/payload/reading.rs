// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{AttachedTuning, EndpointAttached, ModelSelected};
use kernel::{AxError, DialectKind, ModelTag, Payload, Retries};

use super::super::AttachedEndpoint;
use super::super::book::Choice;
use super::super::tuning::EndpointTuning;
use crate::concurrency::MaxInFlight;
use crate::endpoint::{AuthSpec, HeaderValue, ModelFacts};
use crate::market::ModelEntry;
use crate::provider::ceiling::FigureSource;
use crate::provider::registry::ConnectionKind;

pub(crate) fn read_attached(payload: &Payload) -> Result<AttachedEndpoint, AxError> {
    let attached: EndpointAttached = payload.read()?;
    let auth = match (attached.auth, attached.auth_header) {
        (None, _) => AuthSpec::None,
        (Some(value), Some(name)) => AuthSpec::Header { name, value },
        (Some(reference), None) => AuthSpec::Bearer(reference),
    };

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

        models: attached
            .models
            .into_iter()
            .map(|id| ModelFacts {
                id,
                ..ModelFacts::default()
            })
            .collect(),
        probed: attached.probed,
        tuning: tuning_of(attached.tuning.unwrap_or_default())?,
        facts_blob: attached.facts_blob,
    })
}

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

                let value = match HeaderValue::parse(&name, &text) {
                    Ok(value) => value,
                    Err(_) => HeaderValue::Plain(text),
                };
                (name, value)
            })
            .collect(),
        overrides: kept.overrides,
        proxying: kept.proxying.unwrap_or_default(),

        max_in_flight: match kept.max_in_flight.map(MaxInFlight::try_from) {
            Some(Ok(ceiling)) => Some(ceiling),
            Some(Err(_)) | None => None,
        },
        account_retries: kept.account_retries,
        generation: kept.generation,
    })
}

pub(crate) fn read_choice(payload: &Payload) -> Result<(ModelTag, Choice), AxError> {
    let selected: ModelSelected = payload.read()?;
    let max_output_tokens = selected.ceiling();

    let rung = |present: bool, word: Option<&str>| {
        present.then(|| word.map_or(FigureSource::Registered, FigureSource::read))
    };
    let window_from = rung(selected.context_tokens > 0, selected.window_from.as_deref());
    let ceiling_from = rung(
        max_output_tokens.is_some(),
        selected.ceiling_from.as_deref(),
    );

    Ok((
        selected.tag,
        Choice {
            endpoint: selected.endpoint,
            entry: ModelEntry {
                id: selected.model,
                context_tokens: selected.context_tokens,
                window_from,
                max_output_tokens,
                ceiling_from,
                input: selected.input,
                input_price: selected.input_price,
                output_price: selected.output_price,
                cache_read_price: selected.cache_read_price,
                cache_write_price: selected.cache_write_price,
            },
        },
    ))
}
