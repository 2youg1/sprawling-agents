// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::config::{Exposure, ExposureFrom, ExposureTable};
use kernel::event::record::AuditVerdict;
use kernel::{Address, B3Hash, McpReference, Payload, RunId, TimeMs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogAnswer {
    pub building: Address,

    pub run: Option<RunId>,
    pub core_bytes: u64,
    pub index_bytes: u64,
    pub reading_bytes: u64,

    pub always: u32,
    pub dormant: u32,
    pub skills: u32,
    pub changed: u32,

    pub open_sessions: u32,

    pub runs_seen: u32,
    pub tools: Vec<CatalogTool>,
    pub skill_rows: Vec<CatalogSkill>,
    pub references: Vec<McpReference>,
    pub skill_pins: std::collections::BTreeMap<String, B3Hash>,
    pub reading_room: Vec<String>,
    pub city: ExposureTable,
    pub stated: ExposureTable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogTool {
    pub name: String,
    pub namespace: String,
    pub current: Option<CatalogVersion>,
    pub pinned: Option<CatalogVersion>,
    pub change: Option<CatalogChange>,
    pub exposure: Exposure,
    pub from: ExposureFrom,
    pub fallback: Exposure,
    pub runs_used: u32,
    pub last_used: Option<TimeMs>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogVersion {
    pub digest: B3Hash,
    pub hint: String,
    pub bytes: u64,
    pub description: String,
    pub input_schema: Payload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CatalogChange {
    Added,
    Changed,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CatalogSkill {
    pub name: String,
    pub section: String,
    pub disclosure: String,
    pub current_document: Option<String>,
    pub pinned_document: Option<String>,
    pub candidate: Option<B3Hash>,
    pub pinned: Option<B3Hash>,
    pub change: Option<CatalogChange>,
    pub exposure: Exposure,
    pub from: ExposureFrom,
    pub audit: Option<AuditVerdict>,
    pub audit_stale: bool,
    pub runs_used: u32,
    pub last_used: Option<TimeMs>,
}
