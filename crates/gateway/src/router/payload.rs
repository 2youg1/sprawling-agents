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

use kernel::event::record::{AttachedTuning, EndpointAttached, ModelSelected};
use kernel::{AxError, ModelTag, Payload, Proxying, SecretRef};

use crate::concurrency::MaxInFlight;
use crate::endpoint::AuthSpec;
use crate::market::ModelEntry;

use super::attached::AttachedEndpoint;
use super::tuning::EndpointTuning;

mod reading;

pub(crate) use reading::{read_attached, read_choice};

/// The one place an `endpoint_attached` payload is formed.
///
/// # Errors
/// Propagates payload construction failure.
pub fn attached_payload(endpoint: &AttachedEndpoint) -> Result<Payload, AxError> {
    // The reference, never the credential: this is the byte that makes
    // "plaintext only enters the vault" true of the ledger as well.
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
    })
}

/// The registration's `tuning` object, present only when the person
/// settled something, so a line says "nothing was settled" by staying
/// silent about it.
fn settled(tuning: &EndpointTuning) -> Option<AttachedTuning> {
    let kept = AttachedTuning {
        accounts: tuning.accounts.clone(),
        label: tuning.label.clone(),
        timeout_ms: tuning.timeout_ms,
        stream_idle_timeout_ms: tuning.stream_idle_timeout_ms,
        request_max_retries: tuning.request_max_retries.stated(),
        // The default stays silent, the same way every other figure
        // here does.
        proxying: (tuning.proxying != Proxying::default()).then_some(tuning.proxying),
        // A header's value is written as the person's own spelling,
        // which for a credential is the reference and never the key:
        // that is what keeps `endpoint_attached` exportable.
        extra_headers: tuning
            .extra_headers
            .iter()
            .map(|(name, value)| (name.clone(), value.spelled()))
            .collect(),
        overrides: tuning.overrides.clone(),
        max_in_flight: tuning.max_in_flight.map(MaxInFlight::get),
        account_retries: tuning.account_retries,
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

/// The one place a `model_selected` payload is formed.
///
/// `ceiling` says which rung of the ladder supplied the output ceiling,
/// or that the provider picks it: `anthropic.rs` once carried the note
/// that a ceiling invented at the call site truncates runs for a reason
/// that appears nowhere in the account, and this is where that reason
/// appears, in the spelling [`crate::OutputCeiling::word`] owns.
///
/// # Errors
/// Propagates payload construction failure.
pub fn selected_payload(
    tag: ModelTag,
    endpoint: &str,
    entry: &ModelEntry,
    ceiling: Option<crate::OutputCeiling>,
) -> Result<Payload, AxError> {
    Payload::of(&ModelSelected {
        tag,
        endpoint: endpoint.to_owned(),
        model: entry.id.clone(),
        context_tokens: entry.context_tokens,
        max_output_tokens: entry.max_output_tokens.map(kernel::Ceiling::get),
        ceiling_from: ceiling.map(|rung| rung.word().to_owned()),
        input: entry.input,
        input_price: entry.input_price,
        output_price: entry.output_price,
        cache_read_price: entry.cache_read_price,
        cache_write_price: entry.cache_write_price,
    })
}
