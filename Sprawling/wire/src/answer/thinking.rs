// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Effort;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ThinkingOffer {
    pub levels: Vec<Effort>,

    pub on: Switch,

    pub default: Option<Effort>,

    pub default_on: Option<bool>,

    pub words: Vec<EffortWord>,
    pub from: OfferSource,

    pub source: Option<String>,
}

impl ThinkingOffer {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Switch {
    Allowed,
    Refused,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum OfferSource {
    Person,

    Upstream,

    Preset,

    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EffortWord {
    pub effort: Effort,
    pub word: String,
}
