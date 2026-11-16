// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::error::AxCode;
use crate::event::identity::RunId;
use crate::locator::{GitOid, Locator};
use crate::model::Effort;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CheckpointFinding {
    pub path: String,

    pub offset: u64,

    pub len: u64,

    pub line: u64,

    pub reason: String,
}

impl std::fmt::Display for CheckpointFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}+{} ({})",
            self.path, self.line, self.offset, self.len, self.reason
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecretEgressBlocked {
    pub code: String,

    pub scope: Vec<String>,

    pub withheld: Vec<CheckpointFinding>,
}

impl SecretEgressBlocked {
    pub fn checkpoint(scope: Vec<String>, withheld: Vec<CheckpointFinding>) -> Self {
        SecretEgressBlocked {
            code: AxCode::SecretEgress.as_str().to_owned(),
            scope,
            withheld,
        }
    }

    pub fn locations(&self) -> String {
        self.withheld
            .iter()
            .map(CheckpointFinding::to_string)
            .collect::<Vec<String>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CheckpointCommitted {
    JobPinned {
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        job: Locator,
    },

    Committed(Commit),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommitAttribution {
    #[serde(default)]
    pub model: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Commit {
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub oid: GitOid,
    #[serde(flatten)]
    pub by: CommitAttribution,

    #[serde(default)]
    pub scope: Vec<String>,

    #[serde(default)]
    pub files: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub withheld: Vec<CheckpointFinding>,
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
