// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash, Locator, RunId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrefixSlot {
    City,
    Building,
    Resident,
    Run,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixSource {
    pub addr: Address,
    pub kept: u64,
    pub dropped: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixSegment {
    pub slot: PrefixSlot,
    pub hash: B3Hash,
    pub bytes: u64,
    pub text: String,
    pub stored: bool,
    pub sources: Vec<PrefixSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixAnswer {
    pub run: RunId,

    pub segments: Vec<PrefixSegment>,

    pub tools: Option<kernel::event::record::PromptTools>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptShapeAnswer {
    pub run: RunId,

    pub tools: kernel::event::record::ShapePart,

    pub conversation: kernel::event::record::ShapePart,

    pub upper_bound: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ContentAnswer {
    pub locator: Locator,
    pub text: String,
    pub bytes: u64,
    pub truncated: bool,
    pub binary: bool,
}
