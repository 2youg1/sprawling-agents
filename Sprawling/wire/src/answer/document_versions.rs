// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, B3Hash, Seq, TimeMs};
use serde::{Deserialize, Serialize};

pub const VERSIONS_MAX: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VersionsAnswer {
    pub at: Address,
    pub versions: Vec<DocumentVersion>,

    pub more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocumentVersion {
    pub version: B3Hash,

    pub bytes: Option<u64>,

    pub kept: bool,
    pub source: VersionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum VersionSource {
    OnDisk,

    Saved { seq: Seq, at: TimeMs },

    Before { seq: Seq },

    Shipped,
}
