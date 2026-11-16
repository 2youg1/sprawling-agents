// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::SliceVerdict;
use kernel::{Address, B3Hash, Effort, IdemKey, KeepWarm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RangeWrite {
    pub doc: Address,
    pub baseline: B3Hash,
    pub edits: Vec<documents::TextEdit>,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionNaming {
    pub room: Address,
    pub began: kernel::Seq,
    pub name: String,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PolicyChange {
    pub room: Address,
    pub policy: kernel::model::RunPolicy,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoorOpening {
    pub lasting_ms: u64,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoorAnswer {
    pub code: String,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoorStep {
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecretForgetting {
    pub reference: String,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalDecisions {
    pub doc: Address,
    pub decisions: Vec<ProposalDecision>,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalDecision {
    pub proposal: B3Hash,
    pub verdicts: Vec<SliceVerdict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RulesWrite {
    pub building: Address,
    pub base: String,
    pub body: String,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WorkspaceChoiceWrite {
    pub building: Address,

    pub base: String,
    pub endpoint: String,
    pub model: String,
    pub effort: Effort,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildingSettings {
    pub addr: Address,
    pub sandbox: Option<kernel::SandboxLimits>,
    pub mcp: Option<Vec<kernel::McpServer>>,
    pub desktop: Option<String>,

    pub context_second_threshold: Option<u64>,

    pub catalog: Option<kernel::config::ExposureTable>,

    pub mcp_references: Option<Vec<kernel::McpReference>>,

    pub skill_pins: Option<std::collections::BTreeMap<String, kernel::B3Hash>>,

    pub reading_room: Option<Vec<String>>,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CitySettings {
    pub keep_warm: Option<KeepWarm>,
    pub effort: Option<Effort>,

    pub search: Option<kernel::config::SearchConfiguration>,

    pub catalog: Option<kernel::config::ExposureTable>,

    #[serde(default)]
    pub skill_shelves: Option<Vec<String>>,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Carry {
    #[default]
    Nothing,

    Handoff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum HaltScope {
    City,
    Building(Address),
    Workshop(Address),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GovernedDocument {
    Mayor,

    Clerk,

    Preferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum IdentityCard {
    Person {
        user_id: Option<String>,
        imported_from: Option<String>,
        about: Option<String>,
    },

    Mayor {
        name: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SpineDocument {
    Roadmap,

    Memo,

    Handoff,

    Spec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PursuitStep {
    Set { goal: String },
    Pause,
    Resume,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptWrite {
    pub at: Address,
    pub base: B3Hash,
    pub step: PromptStep,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PromptStep {
    Own,

    Inherit,

    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityClosing {
    pub mode: CloseMode,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CloseMode {
    Drain,

    Interrupt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentAdding {
    pub spec_digest: B3Hash,
    pub source: crate::answer::AgentSource,

    pub seat_here: Option<Address>,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentLoginStart {
    pub agent: String,
    pub method: String,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeviceForgetting {
    pub device: crate::answer::DeviceId,
    pub idem: IdemKey,
}
