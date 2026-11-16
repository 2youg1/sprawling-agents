// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::completion::Completion;
use crate::delegation::Difficulty;
use crate::error::AxError;
use crate::event::identity::{RunId, Seq};
use crate::event::kind::EventKind;
use crate::event::who::Who;
use crate::locator::{B3Hash, Locator};
use crate::model::{Effort, ModelRef, RunPolicy};
use crate::origin::Origin;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillPin {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelRouting {

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<Difficulty>,
    pub model: ModelRef,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub policy: Option<B3Hash>,
    pub source: ModelSelectionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ModelSelectionSource {

    Manual,

    Difficulty,

    MediumDefault,

    Choice,

    Main,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunStarted {

    #[serde(default)]
    pub task: String,

    #[serde(default)]
    pub goal: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub job: Option<Locator>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<RunId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<RunId>,

    #[serde(default)]
    pub skills: Vec<SkillPin>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub dispatched_by: Option<Who>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<RunPolicy>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub naming: Option<B3Hash>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<Opening>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<crate::FrozenGeneration>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<ModelRouting>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentRunIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentRunIdentity {

    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub launch_digest: B3Hash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Opening {

    FromJob,

    Inherited,

    WithPerson,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunFrozen {
    pub completion: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Vec<EvidenceCite>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<FreezeCause>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<AxError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum FreezeCause {

    ProcessDied,
}

impl RunFrozen {

    pub fn of(completion: &Completion) -> RunFrozen {
        Self::of_failure(completion, None)
    }

    pub fn of_failure(completion: &Completion, failure: Option<&AxError>) -> RunFrozen {
        let evidence = match completion {
            Completion::Done(evidence) => Some(
                evidence
                    .refs()
                    .iter()
                    .map(|cited| EvidenceCite {
                        seq: cited.seq(),
                        kind: cited.kind(),
                    })
                    .collect(),
            ),
            Completion::Limit | Completion::Cancelled => None,
        };
        RunFrozen {
            completion: completion.name().to_owned(),
            evidence,
            cause: None,
            failure: failure.cloned(),
        }
    }

    pub fn lost() -> RunFrozen {
        RunFrozen {
            completion: Completion::Cancelled.name().to_owned(),
            evidence: None,
            cause: Some(FreezeCause::ProcessDied),
            failure: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EvidenceCite {
    pub seq: Seq,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionOpened {

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Origin>,

    pub carried: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunForked {

    pub from: RunId,

    pub at_seq: Seq,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EvalRun {

    pub probe: String,
    pub version: u32,

    pub predecessor: RunId,

    pub kept: u32,

    pub lost: Vec<u32>,

    pub before: Vec<String>,
    pub after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunPolicyChanged {
    pub policy: RunPolicy,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub by: Who,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionNamed {
    pub began: Seq,
    pub name: String,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
