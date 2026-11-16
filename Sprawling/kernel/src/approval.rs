// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::event::{RunId, Seq, TimeMs};
use crate::locator::Locator;
use crate::registry::ResidentId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalId(String);

impl ApprovalId {
    pub fn of(run: &RunId, seq: Seq) -> ApprovalId {
        ApprovalId(format!("ap-{run}-{:020}", seq.value()))
    }

    pub fn of_sweep(run: &RunId) -> ApprovalId {
        ApprovalId::of(run, Seq::new(u64::MAX))
    }

    pub fn new(raw: impl Into<String>) -> Option<ApprovalId> {
        let raw = raw.into();
        if raw.is_empty() {
            None
        } else {
            Some(ApprovalId(raw))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ApprovalClass {
    Question,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ClusterKey {
    pub class: ApprovalClass,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalItem {
    pub id: ApprovalId,
    pub actor: String,
    pub action_desc: String,
    pub artifact: Locator,
    pub cluster_key: ClusterKey,
    pub created: TimeMs,

    pub tainted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Ruling {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Autonomy {
    Owner,
    Delegate(ResidentId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answerer {
    Human,
    Resident(ResidentId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerVerdict {
    May,

    NotTheDelegate,

    SelfApprovalBarred,
}

pub fn may_answer(autonomy: &Autonomy, item: &ApprovalItem, answerer: &Answerer) -> AnswerVerdict {
    match answerer {
        Answerer::Human => AnswerVerdict::May,
        Answerer::Resident(resident) => {
            let appointed = matches!(autonomy, Autonomy::Delegate(d) if d == resident);
            if !appointed {
                return AnswerVerdict::NotTheDelegate;
            }
            if item.actor == resident.as_str() {
                return AnswerVerdict::SelfApprovalBarred;
            }
            AnswerVerdict::May
        }
    }
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
