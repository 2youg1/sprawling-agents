// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How often each skill and each tool server was used, by whom and on
//! which day (`crates/wire/spec/Reading.lean` D28, D33).
//!
//! Folded from the ledger by `accounting::views::usage`; nothing here
//! decides what counts as a use, which D33 states once.

use kernel::event::record::{AuditVerdict, ShelvedFrom};
use kernel::{Address, B3Hash, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

use super::SkillShelf;

/// Every skill on a shelf, and every skill the ledger saw used, one line
/// each.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillUsageAnswer {
    /// In name order.
    pub skills: Vec<SkillUsageLine>,
}

/// One skill, by name: where it sits now, the contents runs have read,
/// and every time one of them read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillUsageLine {
    pub name: String,
    /// Every shelf that holds the name now, with what it holds there.
    /// Empty for a skill the ledger saw used that no shelf holds any
    /// more.
    pub held: Vec<HeldSkill>,
    /// Each content a run was frozen with, oldest first.
    pub versions: Vec<SkillVersion>,
    /// Every use, in ledger order.
    pub uses: Vec<SkillUse>,
    /// Uses per UTC calendar day, oldest day first.
    pub per_day: Vec<DayCount>,
}

/// One shelf's copy of a skill as it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HeldSkill {
    pub shelf: SkillShelf,
    pub digest: B3Hash,
    pub audit: SkillAudit,
}

/// What the audits on the ledger say about the content a shelf holds now
/// (`city::library::audit_state`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SkillAudit {
    /// No audit was ever had for any content of this skill.
    Unaudited,
    /// The latest audit of this content.
    Audited { verdict: AuditVerdict, at: Seq },
    /// Audits exist, none of them of this content: it changed since, and
    /// the User is asked to audit it again.
    Stale { audited: B3Hash },
}

/// One content of a skill, from the first line that named it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillVersion {
    pub digest: B3Hash,
    /// The first line with this content: its `skill_shelved`, or the
    /// `run_started` that first pinned it.
    pub seq: Seq,
    pub at: TimeMs,
    pub run: RunId,
    #[serde(default)]
    pub author: VersionAuthor,
}

/// Who put one content of a skill on its shelf, read off the ledger in
/// ledger order (`crates/wire/spec/Reading.lean` D33).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum VersionAuthor {
    /// The city shelved it: the `skill_shelved` line and where it came
    /// from.
    Shelved { seq: Seq, from: ShelvedFrom },
    /// The name was shelved before a run first read this content, never
    /// with it: the content was changed outside the city's doors.
    OutsideShelf,
    /// No shelving line names the skill before a run first read this
    /// content: it was on the shelf before the city recorded shelving.
    #[default]
    Unrecorded,
}

/// One read of a skill by a run that pinned it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillUse {
    pub run: RunId,
    /// The room the reading run worked in.
    pub resident: Option<Address>,
    pub seq: Seq,
    pub at: TimeMs,
    /// `guide` for a `describe`, `SKILL.md` for the document, or the
    /// path inside the skill's package.
    pub part: String,
    /// The content the run was frozen with.
    pub digest: B3Hash,
    pub outcome: UseOutcome,
}

/// How the call that used it ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum UseOutcome {
    Ok,
    Failed,
    /// No `tool_result` answers the call: the run was stopped first.
    Unknown,
}

/// How many uses one UTC calendar day holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DayCount {
    /// `YYYY-MM-DD`.
    pub day: String,
    pub count: u32,
}

/// Every tool server a building is configured with, and every one the
/// ledger saw used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpUsageAnswer {
    /// In server order; the line with no server comes last.
    pub servers: Vec<McpServerUsage>,
}

/// One tool server and the tools of it that were used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpServerUsage {
    /// `None` gathers the calls no known server name prefixes: a server
    /// removed before the ledger said whose tool it was.
    pub server: Option<String>,
    /// Whether some building's configuration names it now.
    pub configured: bool,
    /// The tools of it that were used, in tool order.
    pub tools: Vec<McpToolUsage>,
}

/// One tool of a server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpToolUsage {
    /// The tool's name without the server prefix, or the whole name
    /// under the line with no server.
    pub tool: String,
    pub uses: Vec<McpUse>,
    pub per_day: Vec<DayCount>,
}

/// One call of a server's tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpUse {
    pub run: RunId,
    pub resident: Option<Address>,
    pub seq: Seq,
    pub at: TimeMs,
    pub outcome: UseOutcome,
}

/// Which table an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum UsageKind {
    Skills,
    Mcp,
}

/// How an export is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ExportFormat {
    Jsonl,
    Csv,
}

/// One export: every use as one row, in the format asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UsageExportAnswer {
    pub what: UsageKind,
    pub format: ExportFormat,
    /// UTF-8 text the page hands the User as a download.
    pub body: String,
}
