// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, ReleaseVerdict};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReleaseLine {

    pub version: String,

    pub released: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReleaseAnswer {

    Stands {
        mine: ReleaseLine,

        newest: ReleaseLine,
        registries: Vec<RegistryNewest>,
        verdict: ReleaseVerdict,
        update: UpdateHint,
    },

    Unreleased {
        registries: Vec<RegistryNewest>,
        update: UpdateHint,
    },

    Unconfirmed {
        mine: ReleaseLine,
        registries: Vec<RegistryNewest>,
        update: UpdateHint,
    },

    Refused { refusal: AxError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RegistryNewest {
    pub registry: Registry,
    pub reading: RegistryReading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Registry {
    Npm,
    CratesIo,
    Github,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RegistryReading {
    Read { newest: ReleaseLine },
    Refused { refusal: AxError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum InstallChannel {
    Npm,
    Bun,
    Binstall,
    Unknown,
    Package,
    CargoOrBinstall,
    Cargo,
    Archive,
    Aur,
    Source,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UpdateHint {
    pub channel: InstallChannel,

    pub command: Option<String>,

    pub alternatives: Vec<String>,
}
