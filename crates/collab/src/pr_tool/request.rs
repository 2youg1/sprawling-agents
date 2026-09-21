// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The request a resident offers, and the record a rebuild reads back.

use kernel::event::record::CommitAttribution;
use kernel::{AxError, Payload};
use serde::{Deserialize, Serialize};

use crate::workshop::NodeId;

/// A request the city knows about: what it is called, who wrote it, and
/// which branch carries it.
///
/// The field names are the `pr_opened` line's keys: this struct is the
/// one authority for that shape, and both directions go through
/// [`Payload::of`] and `Payload::read` rather than through a hand-written
/// `insert` at the writer and a hand-written `get` at each reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRequest {
    pub node: NodeId,
    pub implementer: String,
    pub branch: String,
    /// The commit the branch stood at when the request was opened. It is
    /// the identity of the work being judged: a verifier who checked one
    /// commit has not vouched for a later one.
    pub commit: String,
}

impl OpenRequest {
    /// The `pr_opened` record, and the shape a rebuild reads back.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn payload(&self) -> Result<Payload, AxError> {
        Payload::of(self)
    }

    /// Reads back what [`payload`](Self::payload) wrote.
    ///
    /// # Errors
    /// Refuses a payload missing any of the four fields, and a node id
    /// this build's grammar refuses.
    pub fn from_payload(data: &Payload) -> Result<OpenRequest, AxError> {
        data.read()
    }

    /// The `pr_merged` record: the request as it was judged, plus what
    /// the merge produced.
    ///
    /// Two commits, two keys. The merge commit used to be written under
    /// `commit`, on top of the commit that was reviewed, so the ledger
    /// held one key for two facts and the audit chain from "what was
    /// verified" to "what landed" broke at the merge.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn merged_payload(
        &self,
        merge_commit: String,
        verified_by: String,
        by: CommitAttribution,
    ) -> Result<Payload, AxError> {
        Payload::of(&MergedRequest {
            node: self.node.clone(),
            implementer: self.implementer.clone(),
            branch: self.branch.clone(),
            reviewed_commit: self.commit.clone(),
            commit: merge_commit,
            verified_by,
            by,
        })
    }
}

/// `pr_merged`: which request landed, which commit was judged, which
/// commit carries it now, and who said so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergedRequest {
    pub node: NodeId,
    pub implementer: String,
    pub branch: String,
    /// The commit a second resident read. Absent from lines written
    /// before this key existed, where the reviewed commit is not
    /// recoverable at all.
    #[serde(default)]
    pub reviewed_commit: String,
    /// The merge commit the trunk now stands at.
    pub commit: String,
    /// The resident who verified the work, never its author.
    pub verified_by: String,
    /// What the merge commit's own trailers carry and this record
    /// cannot say for itself, so "which run wrote this commit" is
    /// answered from the ledger rather than from git.
    #[serde(flatten)]
    pub by: CommitAttribution,
}
