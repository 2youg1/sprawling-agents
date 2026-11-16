// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::event::record::{AuditVerdict, ShelvedFrom};
use kernel::{Address, B3Hash, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

use super::SkillShelf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillUsageAnswer {
    pub skills: Vec<SkillUsageLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillUsageLine {
    pub name: String,

    pub held: Vec<HeldSkill>,

    pub versions: Vec<SkillVersion>,

    pub uses: Vec<SkillUse>,

    pub per_day: Vec<DayCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HeldSkill {
    pub shelf: SkillShelf,
    pub digest: B3Hash,
    pub audit: SkillAudit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SkillAudit {
    Unaudited,

    Audited { verdict: AuditVerdict, at: Seq },

    Stale { audited: B3Hash },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillVersion {
    pub digest: B3Hash,

    pub seq: Seq,
    pub at: TimeMs,
    pub run: RunId,
    #[serde(default)]
    pub author: VersionAuthor,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum VersionAuthor {
    Shelved {
        seq: Seq,
        from: ShelvedFrom,
    },

    OutsideShelf,

    #[default]
    Unrecorded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillUse {
    pub run: RunId,

    pub resident: Option<Address>,
    pub seq: Seq,
    pub at: TimeMs,

    pub part: String,

    pub digest: B3Hash,
    pub outcome: UseOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum UseOutcome {
    Ok,
    Failed,

    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DayCount {
    pub day: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpUsageAnswer {
    pub servers: Vec<McpServerUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpServerUsage {
    pub server: Option<String>,

    pub configured: bool,

    pub tools: Vec<McpToolUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpToolUsage {
    pub tool: String,
    pub uses: Vec<McpUse>,
    pub per_day: Vec<DayCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpUse {
    pub run: RunId,
    pub resident: Option<Address>,
    pub seq: Seq,
    pub at: TimeMs,
    pub outcome: UseOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum UsageKind {
    Skills,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ExportFormat {
    Jsonl,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UsageExportAnswer {
    pub what: UsageKind,
    pub format: ExportFormat,

    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ShellsAnswer {
    pub interpreters: Vec<ShellCalls>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ShellCalls {
    pub interpreter: String,
    pub calls: u64,

    pub failures: BTreeMap<String, u64>,
}
