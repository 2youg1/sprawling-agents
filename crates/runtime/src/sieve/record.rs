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
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassReason {
    BelowFloor,
    NothingShrank,
}

/// The sieve's answer. `Passed` returns the input byte for byte, and
/// carries the account of whatever ran before it decided not to cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sieved {
    Passed {
        text: String,
        reason: PassReason,
        /// The stages that ran, with their counts. `None` below the
        /// floor, where no stage runs at all and the reason is the whole
        /// account.
        account: Option<SieveAccount>,
    },
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

/// The stages a result passes through, in the words the ledger holds.
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageReport {
    pub stage: Stage,
    #[serde(flatten)]
    pub outcome: StageOutcome,
}

/// What the sieve cut out of one result, beyond where it went.
///
/// Four keys or eleven: a result that left the window without the
/// sieve carries the first four and nothing else, which is why this
/// half is one value with a name rather than seven fields that are all
/// present or all absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveAccount {
    /// The filter table this result was cut by.
    pub filter: String,
    pub lines_in: u64,
    pub lines_out: u64,
    /// Every stage the text passed through: applied, no-op, refused or
    /// unavailable. Below the floor no stage ran, and the reason on
    /// [`Sieved::Passed`] is the whole account.
    pub stages: Vec<StageReport>,
}

/// `result_offloaded`: where a result went when it left the window,
/// and what the sieve did to it on the way.
///
/// The one authority for this line's keys. They used to be written by
/// hand twice - once by the sieve and once by the plain offload path -
/// so a reader folding a history met two shapes for one kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultOffloaded {
    /// Where the whole result is kept.
    pub original: Locator,
    /// How long the result was.
    pub len: u64,
    /// How long what the model sees is.
    pub substitute_len: u64,
    /// Where the rest of it was written.
    pub rest_path: String,
    /// The sieve's account, absent on the plain offload path.
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub sieve: Option<SieveAccount>,
}

impl ResultOffloaded {
    /// The payload, through the one door a ledger line is written by.
    ///
    /// # Errors
    /// Propagates whatever `Payload::of` says about the encoding.
    pub fn payload(&self) -> Result<Payload, AxError> {
        Payload::of(self)
    }
}

impl SieveRecord {
    /// The `result_offloaded` account for a result the sieve cut.
    pub fn offloaded(&self) -> ResultOffloaded {
        ResultOffloaded {
            original: self.original.clone(),
            len: self.bytes_in,
            substitute_len: self.bytes_out,
            rest_path: self.rest_path.display().to_string(),
            sieve: Some(SieveAccount {
                filter: self.filter.clone(),
                lines_in: self.lines_in,
                lines_out: self.lines_out,
                stages: self.stages.clone(),
            }),
        }
    }
}
