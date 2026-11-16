// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{AttachedTuning, EndpointAttached, ModelSelected};
use kernel::{AxError, ModelTag, Payload, Proxying, SecretRef};

use crate::concurrency::MaxInFlight;
use crate::endpoint::AuthSpec;
use crate::market::ModelEntry;

use super::attached::AttachedEndpoint;
use super::tuning::EndpointTuning;

mod reading;

pub(crate) use reading::{read_attached, read_choice};

pub fn attached_payload(endpoint: &AttachedEndpoint) -> Result<Payload, AxError> {

    let auth = auth_reference(&endpoint.auth).cloned();
    let auth_header = match &endpoint.auth {
        AuthSpec::Header { name, .. } => Some(name.clone()),
        AuthSpec::Bearer(_) | AuthSpec::None => None,
    };
    Payload::of(&EndpointAttached {
        name: endpoint.name.clone(),
        base_url: endpoint.base_url.clone(),
        dialect: endpoint.dialect,
        auth,
        auth_header,
        models: endpoint.models.iter().map(|row| row.id.clone()).collect(),
        connection_kind: Some(endpoint.connection_kind.as_str().to_owned()),
        probed: endpoint.probed,
        tuning: settled(&endpoint.tuning),
        facts_blob: endpoint.facts_blob,
    })
}

fn settled(tuning: &EndpointTuning) -> Option<AttachedTuning> {
    let kept = AttachedTuning {
        accounts: tuning.accounts.clone(),
        label: tuning.label.clone(),
        timeout_ms: tuning.timeout_ms,
        stream_idle_timeout_ms: tuning.stream_idle_timeout_ms,
        request_max_retries: tuning.request_max_retries.stated(),

        proxying: (tuning.proxying != Proxying::default()).then_some(tuning.proxying),

        extra_headers: tuning
            .extra_headers
            .iter()
            .map(|(name, value)| (name.clone(), value.spelled()))
            .collect(),
        overrides: tuning.overrides.clone(),
        max_in_flight: tuning.max_in_flight.map(MaxInFlight::get),
        account_retries: tuning.account_retries,
        generation: tuning.generation.clone(),
    };
    (kept != AttachedTuning::default()).then_some(kept)
}

pub(crate) fn auth_reference(auth: &AuthSpec) -> Option<&SecretRef> {
    match auth {
        AuthSpec::Bearer(reference) => Some(reference),
        AuthSpec::Header { value, .. } => Some(value),
        AuthSpec::None => None,
    }
}

pub fn selected_payload(
    tag: ModelTag,
    endpoint: &str,
    entry: &ModelEntry,
) -> Result<Payload, AxError> {
    let ceiling_from = match (entry.max_output_tokens, entry.ceiling_from) {
        (Some(_), Some(from)) => Some(from.as_str()),
        (None, Some(_) | None) => Some(crate::OutputCeiling::ProviderDefault.word()),
        (Some(_), None) => None,
    };
    Payload::of(&ModelSelected {
        tag,
        endpoint: endpoint.to_owned(),
        model: entry.id.clone(),
        context_tokens: entry.context_tokens,
        window_from: entry
            .window_from
            .filter(|_| entry.context_tokens > 0)
            .map(|from| from.as_str().to_owned()),
        max_output_tokens: entry.max_output_tokens.map(kernel::Ceiling::get),
        ceiling_from: ceiling_from.map(str::to_owned),
        input: entry.input,
        input_price: entry.input_price,
        output_price: entry.output_price,
        cache_read_price: entry.cache_read_price,
        cache_write_price: entry.cache_write_price,
    })
}
