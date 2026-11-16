// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Performance configuration grammar (`crates/wire/spec/Preference.lean` D30).

use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

/// The placement arm; platform policy belongs to serving::placement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CorePlacement {
    #[serde(rename = "none")]
    Off,
    #[default]
    Soft,
    SoftShares,
    Pinned,
}

/// Whether the core's hot threads stand above normal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CorePriority {
    #[default]
    Raised,
    Normal,
}

/// The person's `[core]` section, read at serving startup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CorePreferences {
    pub placement: CorePlacement,
    pub priority: CorePriority,
    /// Absent means no memory ceiling. Only soft_shares requests it.
    pub memory_bytes: Option<NonZeroU64>,
}
