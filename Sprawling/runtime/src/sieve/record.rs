// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Locator, Payload};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassReason {
    BelowFloor,
    NothingShrank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sieved {
    Passed {
        text: String,
        reason: PassReason,

        account: Option<SieveAccount>,
    },
    Cut(SieveRecord),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SieveRecord {
    pub text: String,
    pub original: Locator,
    pub rest_path: String,
    pub filter: String,
    pub lines_in: u64,
    pub lines_out: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub stages: Vec<StageReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    StripAnsi,
    FoldBlank,
    DedupTemplate,
    DiffPrevious,
    Filter,
    CutLongLine,
    Truncate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum StageOutcome {
    Applied { bytes_before: u64, bytes_after: u64 },
    Noop,

    Rejected { grew_to: u64 },
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageReport {
    pub stage: Stage,
    #[serde(flatten)]
    pub outcome: StageOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveAccount {
    pub filter: String,
    pub lines_in: u64,
    pub lines_out: u64,

    pub stages: Vec<StageReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultOffloaded {
    pub original: Locator,

    pub len: u64,

    pub substitute_len: u64,

    pub rest_path: String,

    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub sieve: Option<SieveAccount>,
}

impl ResultOffloaded {
    pub fn payload(&self) -> Result<Payload, AxError> {
        Payload::of(self)
    }
}

impl SieveRecord {
    pub fn offloaded(&self) -> ResultOffloaded {
        ResultOffloaded {
            original: self.original.clone(),
            len: self.bytes_in,
            substitute_len: self.bytes_out,
            rest_path: self.rest_path.clone(),
            sieve: Some(SieveAccount {
                filter: self.filter.clone(),
                lines_in: self.lines_in,
                lines_out: self.lines_out,
                stages: self.stages.clone(),
            }),
        }
    }
}
