// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a building's admitted tools reach its runs: the shape of a
//! `[catalog]` table and the order its statements are read in.
//!
//! What a table may say and on which rungs is `city`'s answer
//! (`crates/city/spec/ConfigLayers.lean` §8-4g); which tools are fixed
//! and what an unstated tool falls back to is `runtime::catalog`'s
//! (`crates/runtime/spec/Catalog.lean` §8-60). This module holds the one
//! rule between them: a nearer statement covers a farther one, and a
//! statement about a tool covers one about its namespace.
//!
//! Specified by `crates/kernel/spec/Config.lean` §8-22.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// How one admitted tool reaches a building's sessions.
///
/// Three values, the truncation lock's three tiers: `Core` travels in
/// every request's tool list, `Dormant` is one line of the dormant index
/// under its namespace and runs through `tool`, and `Off` is not admitted
/// at all, so it costs no byte anywhere and no call reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Exposure {
    Core,
    Dormant,
    Off,
}

/// One layer's `[catalog]` table, as written: tool name to exposure,
/// and namespace to exposure. Both maps empty is a layer that says
/// nothing, which is how most layers stay.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ExposureTable {
    #[serde(default)]
    pub tools: BTreeMap<String, Exposure>,
    #[serde(default)]
    pub namespaces: BTreeMap<String, Exposure>,
}

impl ExposureTable {
    /// Whether this table states nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.namespaces.is_empty()
    }

    /// What this table says about `tool`, filed under `namespace`: the
    /// tool's own key first, then its namespace's.
    fn said(&self, tool: &str, namespace: &str) -> Option<Exposure> {
        self.tools
            .get(tool)
            .or_else(|| self.namespaces.get(namespace))
            .copied()
    }
}

/// Where the exposure a tool was given came from. Closed, because a page
/// that shows the source must be able to name every one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ExposureFrom {
    /// The tool cannot be moved: the catalog's own door, or the one that
    /// opens what the index lists.
    Fixed,
    /// No layer states it, so the tool takes the catalog's default.
    Default,
    /// The city's own layer states it, for the tool or its namespace.
    City,
    /// The building's layer states it, for the tool or its namespace.
    Building,
}

/// The two `[catalog]` tables a run is frozen with. A room's layer
/// states none: the ladder refuses it there, because the tool array sits
/// ahead of the system prompt in a provider's cache and rooms that
/// differed would split one building's cache.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogExposure {
    #[serde(default)]
    pub city: ExposureTable,
    #[serde(default)]
    pub building: ExposureTable,
}

impl CatalogExposure {
    /// What the two tables state for `tool`, filed under `namespace`,
    /// and which layer said it: the building's tool key, the building's
    /// namespace key, the city's tool key, the city's namespace key, in
    /// that order. `None` when neither table names the tool or its
    /// namespace, and the catalog's default then decides.
    #[must_use]
    pub fn stated(&self, tool: &str, namespace: &str) -> Option<(Exposure, ExposureFrom)> {
        self.building
            .said(tool, namespace)
            .map(|exposure| (exposure, ExposureFrom::Building))
            .or_else(|| {
                self.city
                    .said(tool, namespace)
                    .map(|exposure| (exposure, ExposureFrom::City))
            })
    }
}
