// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    Address, McpServer, NodeId, Progress, PursuitState, PursuitVerdict, RoadmapStatus,
    SandboxLimits,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingProgress {
    pub addr: Address,
    pub progress: Progress,
    pub problems: Vec<String>,

    pub blocked: Vec<BlockedLine>,

    pub ready: u32,

    pub adoptable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TemplateAnswer {
    pub template: crate::TemplateName,
    pub addr: Address,
    pub rules: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BlockedLine {
    pub source: NodeId,

    pub line: String,

    pub waiting: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PlanRow {
    pub node: NodeId,
    pub item: String,
    pub status: RoadmapStatus,

    pub share_ppb: u64,
    pub needs: Vec<NodeId>,

    pub ready: bool,

    pub leaf: bool,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PursuitLine {
    pub addr: Address,
    pub goal: String,
    pub state: PursuitState,

    pub verdict: PursuitVerdict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingDoc {
    pub name: String,
    pub text: String,
    pub bytes: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArchiveLine {
    pub kind: String,

    pub day: u64,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingAnswer {
    pub addr: Address,
    pub progress: Progress,

    pub problems: Vec<String>,

    pub plan: Vec<PlanRow>,

    pub blocked: Vec<BlockedLine>,
    pub rooms: Vec<String>,
    pub docs: Vec<BuildingDoc>,
    pub archive: Vec<ArchiveLine>,

    pub sandbox: Option<SandboxLimits>,
    pub mcp: Vec<McpServer>,
}
