// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, GitOid, Locator, RunId, Seq, TimeMs, Tokens, UsdMicros};
use serde::{Deserialize, Serialize};

use super::Note;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Outcome {

    Waiting,

    Answered,

    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Timing {

    Measured,

    Unmeasured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Output {

    pub head: String,

    pub cut: usize,

    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub pinned: Option<Locator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Used {
    pub input: Tokens,
    pub output: Tokens,

    #[serde(default)]
    pub cached: Option<Tokens>,

    #[serde(default)]
    pub cache_write: Option<Tokens>,

    #[serde(default)]
    pub first_us: Option<u64>,

    #[serde(default)]
    pub took_us: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Call {

    pub tool: String,

    pub subject: Option<String>,

    pub arguments: Option<Output>,
    pub outcome: Outcome,

    pub at: Seq,

    pub output: Option<Output>,

    pub called: TimeMs,

    pub answered: Option<TimeMs>,

    pub timing: Timing,

    pub effect: Option<kernel::Effect>,

    pub render: Option<kernel::RenderIntent>,

    #[serde(default)]
    pub exit_code: Option<i64>,

    #[serde(default)]
    pub took_us: Option<u64>,

    #[serde(default)]
    pub landing: Option<kernel::event::record::Landing>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Turn {

    pub number: u32,

    pub opened: Seq,

    pub t: TimeMs,

    pub timing: Timing,

    pub first_at: Option<TimeMs>,

    #[serde(default)]
    pub returned: Option<TimeMs>,

    pub model: Option<String>,

    pub said: Option<String>,

    pub thought: Option<String>,

    pub spent: Option<UsdMicros>,
    pub used: Option<Used>,

    pub stopped: Option<String>,
    pub calls: Vec<Call>,

    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Opening {
    pub task: String,
    pub goal: String,
    pub at: TimeMs,

    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub dispatched_by: Option<kernel::event::Who>,

    #[serde(default)]
    pub policy: Option<kernel::RunPolicy>,

    #[serde(default)]
    pub effort: Option<kernel::Effort>,

    #[serde(default)]
    pub generation: Option<kernel::FrozenGeneration>,

    #[serde(default)]
    pub names: Option<FrozenNames>,

    #[serde(default)]
    pub parent: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FrozenNames {

    pub mayor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Closing {
    pub completion: String,
    pub at: TimeMs,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<AxError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RoundsAnswer {
    pub run: RunId,
    pub turns: Vec<Turn>,

    pub opened_at: Option<GitOid>,

    pub opening: Option<Opening>,

    pub closing: Option<Closing>,

    #[serde(default)]
    pub worktree: Option<String>,
}
