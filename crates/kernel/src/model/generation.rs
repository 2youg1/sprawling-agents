// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Generation values shared by settings, the Ledger and canonical calls.
//! Sampling numbers stay as text until the provider request boundary, so
//! no Ledger payload or configuration file acquires a float. Specified by
//! `crates/kernel/spec/Model.lean` §8-24, kernel D124.

use serde::{Deserialize, Serialize};

use super::Ceiling;

/// Whether one generation parameter is known to be accepted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ParameterSupport {
    /// Nobody has stated the capability; an explicit value is permitted.
    #[default]
    Unknown,
    /// The parameter is accepted.
    Supported,
    /// The parameter is rejected and must not carry a typed value.
    Unsupported,
}

/// Who stated a parameter's support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ParameterSource {
    /// The person explicitly corrected the capability.
    Person,
    /// The provider's model list stated it in a machine-readable field.
    Upstream,
    /// A preset row cited the provider's documentation.
    Preset,
}

/// The person's corrections to three sampling capabilities. An absent
/// field leaves the upstream and preset rungs free to answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GenerationSupport {
    #[serde(default)]
    pub temperature: Option<ParameterSupport>,
    #[serde(default)]
    pub top_p: Option<ParameterSupport>,
    #[serde(default)]
    pub top_k: Option<ParameterSupport>,
}

/// Sampling defaults for new sessions. Every number remains the exact
/// text the person entered; absence is never zero.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GenerationSettings {
    #[serde(default)]
    pub temperature: Option<String>,
    #[serde(default)]
    pub top_p: Option<String>,
    #[serde(default)]
    pub top_k: Option<String>,
}

/// One model's generation editor contents at one endpoint. `support`
/// holds only the person's statements; resolved answers also carry their
/// source on the wire.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelGeneration {
    pub model: String,
    #[serde(default)]
    pub settings: GenerationSettings,
    /// Preferred output ceiling for a new session, distinct from the
    /// model's hard output ceiling.
    #[serde(default)]
    pub output_tokens: Option<Ceiling>,
    #[serde(default)]
    pub support: GenerationSupport,
}

/// The generation a session actually froze: typed sampling text, the
/// effective output ceiling, and the raw request overrides applied last.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FrozenGeneration {
    #[serde(default)]
    pub settings: GenerationSettings,
    #[serde(default)]
    pub max_tokens: Option<Ceiling>,
    #[serde(default)]
    pub overrides: Vec<(String, String)>,
}
