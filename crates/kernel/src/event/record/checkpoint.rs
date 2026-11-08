// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a checkpoint records: the job a dispatch pinned, or the commit
//! a checkpoint raised.

use serde::{Deserialize, Serialize};

use crate::error::AxCode;
use crate::event::identity::RunId;
use crate::locator::{GitOid, Locator};
use crate::model::Effort;

/// One checkpoint scan finding. It identifies a place and an evidence
/// class, never the matched bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CheckpointFinding {
    /// Repository-relative path of the file scanned.
    pub path: String,
    /// Byte offset of the match within that file, from zero.
    pub offset: u64,
    /// Match length in bytes.
    pub len: u64,
    /// One-based source line containing the match's first byte.
    pub line: u64,
    /// A provider's known shape name, or `high entropy`.
    pub reason: String,
}

impl std::fmt::Display for CheckpointFinding {
    /// The actionable location in a refusal; deliberately has no text field.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}+{} ({})",
            self.path, self.line, self.offset, self.len, self.reason
        )
    }
}

/// A checkpoint refused because at least one known provider shape matched.
/// The same value is the storage error's report and the ledger payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecretEgressBlocked {
    /// Stable refusal code, always `E_SECRET_EGRESS` for this constructor.
    pub code: String,
    /// The checkpoint's staging scope.
    pub scope: Vec<String>,
    /// Every finding, with positions and reasons only.
    pub withheld: Vec<CheckpointFinding>,
}

impl SecretEgressBlocked {
    /// The report of one refused checkpoint.
    pub fn checkpoint(scope: Vec<String>, withheld: Vec<CheckpointFinding>) -> Self {
        SecretEgressBlocked {
            code: AxCode::SecretEgress.as_str().to_owned(),
            scope,
            withheld,
        }
    }

    /// Locations and reasons for the error's subject, with no matched bytes.
    pub fn locations(&self) -> String {
        self.withheld
            .iter()
            .map(CheckpointFinding::to_string)
            .collect::<Vec<String>>()
            .join(" ")
    }
}

/// `checkpoint_committed` carries two facts that are not the same fact,
/// and this enum states which one a line holds instead of leaving a
/// reader to infer it from which keys are absent.
///
/// A dispatch opens a run by pinning the job it came out of; a checkpoint
/// raises a git commit before a wave. Both were written under this one
/// kind before these structs existed, and re-spelling a ledger already
/// on disk is not available, so the two shapes stay under one kind and
/// are told apart by their keys. The variants are untagged: each one
/// writes exactly the object its hand-written predecessor wrote, with
/// no discriminator added.
///
/// **The pin should be its own kind.** It answers "which job is this
/// run for", which no commit answers, and every reader of this kind
/// starts by discarding it. Splitting it needs a new `EventKind` and a
/// ledger version, so it is recorded here rather than done here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CheckpointCommitted {
    /// The dispatch pin: the job a run was opened for, and no commit.
    JobPinned {
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        job: Locator,
    },
    /// A checkpoint or a landing: the commit, and the facts the record
    /// cannot state for itself.
    Committed(Commit),
}

/// Which session made a commit, as the ledger states it.
///
/// One fact with one home, carried by both kinds of record that name a
/// commit: `checkpoint_committed` from a checkpoint and `pr_merged` from a
/// review. The run and the actor are the record's own identity and are
/// not repeated here; the model and the effort are, because until they
/// are written here they exist only on the git commit's trailers, and
/// the trailers are the projection rather than the authority.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommitAttribution {
    /// The endpoint's own model id, empty when the caller did not know
    /// it. Absent in every record written before the key existed, which
    /// reads back the same as unknown.
    #[serde(default)]
    pub model: String,
    /// Absent leaves the choice to the provider, which is not the same
    /// fact as [`Effort::None`] asking it not to think. A checkpoint writes
    /// `Effort::None` for both, because that is the word its git
    /// trailer has always carried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    /// The run this one replaced, when it is a successor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<RunId>,
}

/// The commit a checkpoint raised, and what it staged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Commit {
    /// The commit object this checkpoint wrote.
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub oid: GitOid,
    #[serde(flatten)]
    pub by: CommitAttribution,
    /// Which parts of the city the checkpoint staged, empty for the whole of
    /// it. Written even when empty, as the hand-written writer did.
    #[serde(default)]
    pub scope: Vec<String>,
    /// The paths this commit touched.
    #[serde(default)]
    pub files: Vec<String>,
    /// Entropy-only lines committed but withheld from patch readers. Old
    /// records lack this key and read as having none; clean commits omit it.
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
