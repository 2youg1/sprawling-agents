// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the sieve answers with: the text the model sees, the way back
//! to the original, and the account of every stage — kept, no-op,
//! refused, or unavailable — so a filter that stops working can be
//! read off the Ledger afterwards. The payload holds integers and
//! strings only.

use std::path::PathBuf;

use kernel::{AxError, Locator, Payload};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassReason {
    BelowFloor,
    NothingShrank,
}

/// The sieve's answer. `Passed` returns the input byte for byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sieved {
    Passed { text: String, reason: PassReason },
    Cut(SieveRecord),
}

/// What the model sees, the way back, and the account of every stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SieveRecord {
    pub text: String,
    pub original: Locator,
    pub rest_path: PathBuf,
    pub filter: String,
    pub lines_in: u64,
    pub lines_out: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub stages: Vec<StageReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    StripAnsi,
    FoldBlank,
    DedupTemplate,
    DiffPrevious,
    Filter,
    CutLongLine,
    Truncate,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::StripAnsi => "strip_ansi",
            Stage::FoldBlank => "fold_blank",
            Stage::DedupTemplate => "dedup_template",
            Stage::DiffPrevious => "diff_previous",
            Stage::Filter => "filter",
            Stage::CutLongLine => "cut_long_line",
            Stage::Truncate => "truncate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageOutcome {
    Applied {
        bytes_before: u64,
        bytes_after: u64,
    },
    Noop,
    /// The stage would have grown the text, so its result was refused.
    Rejected {
        grew_to: u64,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReport {
    pub stage: Stage,
    pub outcome: StageOutcome,
}

impl StageReport {
    fn json(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "stage".to_owned(),
            Value::String(self.stage.name().to_owned()),
        );
        match &self.outcome {
            StageOutcome::Applied {
                bytes_before,
                bytes_after,
            } => {
                map.insert("outcome".to_owned(), json!("applied"));
                map.insert("bytes_before".to_owned(), json!(bytes_before));
                map.insert("bytes_after".to_owned(), json!(bytes_after));
            }
            StageOutcome::Noop => {
                map.insert("outcome".to_owned(), json!("noop"));
            }
            StageOutcome::Rejected { grew_to } => {
                map.insert("outcome".to_owned(), json!("rejected"));
                map.insert("grew_to".to_owned(), json!(grew_to));
            }
            StageOutcome::Unavailable { reason } => {
                map.insert("outcome".to_owned(), json!("unavailable"));
                map.insert("reason".to_owned(), json!(reason));
            }
        }
        Value::Object(map)
    }
}

impl SieveRecord {
    /// The `result_offloaded` payload: the original's locator, the
    /// substitute's length, and the stage account. Integers only.
    pub fn payload(&self) -> Result<Payload, AxError> {
        let mut map = Map::new();
        map.insert(
            "original".to_owned(),
            Value::String(self.original.to_string()),
        );
        map.insert("len".to_owned(), json!(self.bytes_in));
        map.insert("substitute_len".to_owned(), json!(self.bytes_out));
        map.insert(
            "rest_path".to_owned(),
            Value::String(self.rest_path.display().to_string()),
        );
        map.insert("filter".to_owned(), Value::String(self.filter.clone()));
        map.insert("lines_in".to_owned(), json!(self.lines_in));
        map.insert("lines_out".to_owned(), json!(self.lines_out));
        map.insert(
            "stages".to_owned(),
            Value::Array(self.stages.iter().map(StageReport::json).collect()),
        );
        Payload::new(map)
    }
}
