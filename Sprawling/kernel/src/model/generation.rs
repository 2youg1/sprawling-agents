// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use super::Ceiling;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ParameterSupport {
    #[default]
    Unknown,

    Supported,

    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ParameterSource {
    Person,

    Upstream,

    Preset,
}

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelGeneration {
    pub model: String,
    #[serde(default)]
    pub settings: GenerationSettings,

    #[serde(default)]
    pub output_tokens: Option<Ceiling>,
    #[serde(default)]
    pub support: GenerationSupport,
}

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
