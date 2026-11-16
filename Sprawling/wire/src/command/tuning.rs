// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Proxying;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HeaderPair {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BodyOverride {
    pub pointer: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointTuning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<kernel::event::record::ProviderAccount>>,

    pub label: Option<String>,

    pub timeout_ms: Option<u64>,

    pub request_max_retries: Option<u32>,

    pub stream_idle_timeout_ms: Option<u64>,
    pub headers: Vec<HeaderPair>,
    pub overrides: Vec<BodyOverride>,

    pub proxying: Option<Proxying>,

    pub max_in_flight: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_retries: Option<kernel::account_recovery::AccountRetries>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generation: Vec<kernel::ModelGeneration>,
}
