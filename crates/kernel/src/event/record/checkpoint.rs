// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a checkpoint records: the job a dispatch pinned, or the commit
//! a fence raised.

use serde::{Deserialize, Serialize};

use crate::event::identity::RunId;
use crate::locator::{GitOid, Locator};
use crate::model::Effort;

/// `checkpoint_committed` carries two facts that are not the same fact,
/// and this enum states which one a line holds instead of leaving a
/// reader to infer it from which keys are absent.
///
/// A dispatch opens a run by pinning the job it came out of; a fence
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
    /// A fence or a landing: the commit, and the facts the record
    /// cannot state for itself.
    Committed(Commit),
}

/// Which session made a commit, as the ledger states it.
///
/// One fact with one home, carried by both kinds of record that name a
/// commit: `checkpoint_committed` from a fence and `pr_merged` from a
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
    /// fact as [`Effort::None`] asking it not to think. A fence writes
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
    /// Which parts of the city the fence staged, empty for the whole of
    /// it. Written even when empty, as the hand-written writer did.
    #[serde(default)]
    pub scope: Vec<String>,
    /// The paths this commit touched.
    #[serde(default)]
    pub files: Vec<String>,
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
