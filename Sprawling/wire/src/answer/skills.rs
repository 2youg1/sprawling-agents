// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash, RunId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SkillShelf {
    Library,

    Building(Address),

    External { index: u32, path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillLine {
    pub name: String,

    pub section: String,

    pub shelf: SkillShelf,

    pub disclosure: String,

    pub hash: B3Hash,

    pub admitted: bool,

    pub pinned_by: Vec<RunId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillsAnswer {
    pub building: Address,

    pub skills: Vec<SkillLine>,

    pub missing: Vec<String>,

    #[serde(default)]
    pub external_shelves: Vec<String>,
}
