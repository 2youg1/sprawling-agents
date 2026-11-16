// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the settings page reads back about attached endpoints.

use kernel::{Ceiling, DialectKind, ModelTag, ServerLabel};
use serde::{Deserialize, Serialize};

use super::ModelFactsSummary;

/// What the settings page reads back: what is attached, and what each
/// tag currently points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointsAnswer {
    pub endpoints: Vec<EndpointSummary>,
    pub chosen: Vec<ChosenSummary>,
}

/// One attached endpoint, as a reader may see it. No credential appears
/// here in any form; `has_credential` answers the only question a page
/// needs, which is whether one was enrolled at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointSummary {
    pub name: String,
    /// What to call it on screen. The id it was filed under when the
    /// person gave it no other name, so a page never has to decide what
    /// to show when a label is missing.
    pub label: String,
    pub base_url: String,
    pub dialect: DialectKind,
    /// How this endpoint is connected, as one flat word:
    /// `openai_compat`, `responses` or `anthropic_native`.
    ///
    /// Resolved once when the endpoint was attached and read back
    /// here. `dialect` beside it answers a narrower question - which
    /// request writer runs - and two connections can share a writer
    /// while being different registrations, so a page given only the
    /// writer cannot say which one a person set up. The word's one
    /// authority is `gateway::provider::registry::ConnectionKind`.
    pub connection_kind: String,
    /// The models this endpoint serves, with what it said about each.
    pub models: Vec<ModelFactsSummary>,
    /// Whether calls to it stay on this machine, which is the only thing
    /// a confidential building may use.
    pub local: bool,
    pub has_credential: bool,
    /// What the person attached this endpoint with, read back in the
    /// shape `AttachEndpoint` carries it: a page that changes the account
    /// list sends this back whole with only `accounts` replaced, so the
    /// figures, headers and overrides nobody touched stay as they were
    /// (`crates/wire/spec/Answer/Endpoints.lean` §8-85, wire D49).
    pub tuning: crate::EndpointTuning,
    /// Whether each listed account's key is there, one row per id in the
    /// order of `tuning.accounts`; empty for an endpoint registered
    /// without an account list.
    pub account_status: Vec<AccountStatus>,
}

/// Whether one account's key is there, by the account's id. Says nothing
/// of the key's value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccountStatus {
    pub id: ServerLabel,
    pub key: KeyState,
}

/// Where one account's key stands, decided by the city so a page draws a
/// word rather than reading two flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum KeyState {
    /// The account names no reference: its requests carry no key.
    Anonymous,
    /// The vault holds a key under the account's reference.
    Stored,
    /// Neither the vault nor an environment variable has one, and the
    /// page may store one.
    Missing,
    /// An environment variable supplies it; the page cannot change it.
    Environment,
    /// An environment variable is set for it and does not read as text:
    /// it shades the vault, so the key cannot be used either way.
    EnvironmentUnusable,
    /// This city has no vault open, so nothing was looked up.
    Unread,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChosenSummary {
    pub tag: ModelTag,
    pub endpoint: String,
    pub model: String,
    /// Absent when no row states this model's ceiling — which the
    /// settings page shows as a field to fill rather than as a number.
    pub max_output_tokens: Option<Ceiling>,
}
