// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::config::SearchConfiguration;
use kernel::{Address, Effort, KeepWarm, Proxying, ServerLabel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ConfigLayer {
    Default,

    City,

    Building,

    Resident,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettledEffort {
    pub effort: Effort,
    pub from: ConfigLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettledSecond {
    pub percent: u64,
    pub from: ConfigLayer,
    pub domain: SecondDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecondDomain {
    pub min: u64,
    pub max: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TuningDefaults {
    pub from: ConfigLayer,

    pub timeout_ms: u64,

    pub request_max_retries: Option<u32>,

    pub stream_idle_timeout_ms: Option<u64>,

    pub proxying: Proxying,

    pub account_retries: kernel::account_recovery::AccountRetries,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConfigAnswer {
    pub addr: Address,

    pub effort: Option<SettledEffort>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city_effort: Option<Effort>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city_keep_warm: Option<KeepWarm>,

    pub second: SettledSecond,

    #[serde(default)]
    pub first: Option<u64>,

    #[serde(default)]
    pub context_lock_tokens: Option<u64>,
    pub tuning: TuningDefaults,

    pub search: SettledSearch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SettledSearch {
    pub configuration: SearchConfiguration,

    pub from: ConfigLayer,

    pub city: Option<SearchConfiguration>,

    pub default_url: String,

    pub account_status: Vec<SupplierAccounts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SupplierAccounts {
    pub supplier: ServerLabel,
    pub accounts: Vec<super::AccountStatus>,
}
