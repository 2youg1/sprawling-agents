// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::locator::Locator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityInitialized {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingCreated {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub addr: Address,

    pub template: String,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub adopted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingConfigured {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub addr: Address,
    pub sandbox: bool,
    pub mcp: bool,
    pub desktop: bool,
    pub context: bool,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub catalog: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CancelReceived {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HandoffWritten {
    #[cfg_attr(feature = "schema", schemars(with = "Vec<String>"))]
    pub must_read: Vec<Locator>,
    pub overview: String,
    pub progress: String,
    pub context: String,
    pub next_step: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WatchdogFired {
    #[serde(flatten)]
    pub action: FiredAction,

    pub corrections: u32,

    pub provider_failures: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FiredAction {

    Steer { text: String },

    BackOff {
        until_ms: u64,
        code: String,
        subject: String,
    },

    Switch {
        to: crate::ServerLabel,
        code: String,
        subject: String,
    },

    Freeze { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GateChecked {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PolicyChanged {
    pub id: String,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
