// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash};
use serde::{Deserialize, Serialize};

use super::prefix::PrefixSlot;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LayersAnswer {
    pub addr: Address,
    pub documents: Vec<LayerDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LayerDocument {
    pub at: Address,
    pub version: B3Hash,
    pub slot: PrefixSlot,
    pub kind: PromptKind,
    pub layer: PromptLayer,
    pub state: LayerState,

    pub shipped: Option<ShippedDefault>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PromptKind {

    City,

    Conventions,

    Identity,

    ModelNote { provider: String, model: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PromptLayer {
    City,
    Building,
    Room,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LayerState {

    Sent { bytes: u64 },

    Replaced { by: Address },

    Absent,

    Withheld { why: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ShippedDefault {
    pub version: B3Hash,
    pub standing: DefaultStanding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DefaultStanding {
    Absent,
    Current,
    Earlier,
    Edited,
}
