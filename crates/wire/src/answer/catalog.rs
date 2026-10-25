// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How each tool a building admits reaches its runs
//! (`crates/wire/spec/Answer/Catalog.lean` §8-99).
//!
//! The rows are the tools the building's newest run was frozen with,
//! because assembling a workbench to answer a page would shake hands with
//! every server and may start a browser. What each row is exposed as is
//! read against the `[catalog]` tables on disk now, so the page shows
//! what the next run gets and edits the tables that decide it.

use kernel::config::{Exposure, ExposureFrom, ExposureTable};
use kernel::{Address, RunId, TimeMs};
use serde::{Deserialize, Serialize};

/// The tools of one building, and the two tables a page edits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogAnswer {
    pub building: Address,
    /// The newest run of this building whose prompt was frozen; `None`
    /// before the first, and then `tools` is empty.
    pub run: Option<RunId>,
    /// Bytes of the core tool array that run carried.
    pub core_bytes: u64,
    /// Bytes of the dormant index in that run's resident segment.
    pub index_bytes: u64,
    /// How many of this building's newest runs the evidence of use was
    /// counted over: at most the window, fewer in a younger building.
    pub runs_seen: u32,
    /// One row per tool that run's assembler built, in name order.
    pub tools: Vec<CatalogTool>,
    /// The city's own `[catalog]` table, as written.
    pub city: ExposureTable,
    /// This building's `[catalog]` table, as written.
    pub stated: ExposureTable,
}

/// One tool of a building.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogTool {
    pub name: String,
    pub namespace: String,
    /// The first sentence of its description, as the dormant index cuts it.
    pub hint: String,
    /// Bytes of its definition: what it adds to every request while core.
    pub bytes: u64,
    /// What the next run will expose it as, and which rung decided it.
    pub exposure: Exposure,
    pub from: ExposureFrom,
    /// What it is exposed as when neither table states it: the default,
    /// or the fixed exposure when `from` is `Fixed`.
    pub fallback: Exposure,
    /// How many of the `runs_seen` runs called it at least once.
    pub runs_used: u32,
    /// When it was last called in this building, within those runs.
    pub last_used: Option<TimeMs>,
}
