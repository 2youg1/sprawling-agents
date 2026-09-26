// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the settings page reads back about attached endpoints.

use kernel::{Ceiling, DialectKind, ModelTag};
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
    /// `openai_compat`, `responses`, `anthropic_native`, or the name of
    /// the first-party harness whose subscription pays for it.
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
