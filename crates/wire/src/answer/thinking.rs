// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which thinking levels one (Endpoint, model) offers, and where that
//! answer came from (`crates/wire/spec/Answer/Endpoints.lean` §8-92).

use kernel::Effort;
use serde::{Deserialize, Serialize};

/// The thinking levels one (Endpoint, model) offers, as
/// `gateway::provider::thinking` resolved them.
///
/// Sourced rather than said: `from` names the rung of the ladder that
/// answered, so a page never mistakes this for the upstream's own words,
/// which are the other fields of the model row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThinkingOffer {
    /// In the city's fixed ascending order; never `none`, because turning
    /// thinking off is not a level. Empty means no thinking control.
    pub levels: Vec<Effort>,
    /// Whether a model with a switch and no levels may be asked to think.
    pub on: Switch,
    /// The level the upstream says it uses when asked for none.
    pub default: Option<Effort>,
    /// Whether the upstream says it thinks when sent nothing.
    pub default_on: Option<bool>,
    /// The upstream's own word for a level, where it differs from the
    /// city's spelling.
    pub words: Vec<EffortWord>,
    pub from: OfferSource,
    /// Where a preset rung's statement is written.
    pub source: Option<String>,
}

impl ThinkingOffer {
    /// The offer nothing states: no levels, nothing known about the
    /// switch or the default.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            levels: Vec::new(),
            on: Switch::Unknown,
            default: None,
            default_on: None,
            words: Vec::new(),
            from: OfferSource::Unknown,
            source: None,
        }
    }
}

/// Whether one setting is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Switch {
    Allowed,
    Refused,
    Unknown,
}

/// The rung of the ladder an offer came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum OfferSource {
    /// The person stated the levels when attaching the endpoint.
    Person,
    /// The upstream's model listing stated them.
    Upstream,
    /// The vendor's documentation, as the preset table records it.
    Preset,
    /// Nothing states them.
    Unknown,
}

/// The upstream's own word for one level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EffortWord {
    pub effort: Effort,
    pub word: String,
}
