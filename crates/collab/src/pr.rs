// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Getting a node's work into the building, with the verification taken
//! out of the implementer's hands.
//!
//! The phases are types: a pull request that has not been verified has
//! no method that merges it, so "verified before merged" is not a rule
//! anybody has to remember. Verification itself is not re-decided here -
//! an [`Artifact`](crate::fanin::Artifact) already carries the fact that
//! somebody other than the producer ran the done check, and this module
//! reuses that judgment rather than making a second one.
//!
//! What this module adds is the match between a request and the work
//! offered for it. The ledger records are not written here: the one
//! authority for a pull request's payload is
//! [`OpenRequest`](crate::pr_tool::OpenRequest), which carries the
//! `commit` field a rebuild needs and this module never had.

use kernel::{AxCode, AxError};

use crate::fanin::Artifact;
use crate::workshop::NodeId;

/// A request to bring one node's branch into the building.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pr<S> {
    node: NodeId,
    implementer: String,
    branch: String,
    state: S,
}

/// Opened, not yet answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Open;

/// Answered by verification, and mergeable for that reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    by: String,
}

impl Pr<Open> {
    /// Opens a request for a node's branch.
    ///
    /// # Errors
    /// Refuses an unnamed implementer: the one thing this flow is built
    /// to know is who must not be the verifier.
    pub fn open(node: NodeId, implementer: String, branch: String) -> Result<Pr<Open>, AxError> {
        if implementer.trim().is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "open a pull request",
                node.as_str().to_owned(),
            )
            .with_recovery(
                "name the implementer; this flow exists to keep them out of their own review",
            ));
        }
        Ok(Pr {
            node,
            implementer,
            branch,
            state: Open,
        })
    }

    /// Accepts verification, in the form of an artifact somebody else
    /// produced by running the node's done check.
    ///
    /// # Errors
    /// Refuses an artifact for a different node, and one whose verifier
    /// is the implementer. The second refusal is nearly unreachable -
    /// `Artifact` cannot be built by its own producer - and it is kept
    /// because "nearly" is doing the work of a review nobody performs.
    pub fn verified(self, artifact: &Artifact) -> Result<Pr<Verified>, AxError> {
        if artifact.node() != &self.node {
            return Err(AxError::failure(
                AxCode::EvidenceMissing,
                "verify a pull request",
                format!(
                    "the artifact is for node {}, the request is for {}",
                    artifact.node().as_str(),
                    self.node.as_str()
                ),
            )
            .with_recovery("verify the node this request is for"));
        }
        if artifact.verified_by() == self.implementer {
            return Err(AxError::failure(
                AxCode::EvidenceMissing,
                "verify a pull request",
                format!("{} verified their own work", self.implementer),
            )
            .with_recovery("have a test resident run the done check"));
        }
        Ok(Pr {
            node: self.node,
            implementer: self.implementer,
            branch: self.branch,
            state: Verified {
                by: artifact.verified_by().to_owned(),
            },
        })
    }
}

impl Pr<Verified> {
    /// The resident that ran the check.
    #[must_use]
    pub fn verified_by(&self) -> &str {
        &self.state.by
    }
}

impl<S> Pr<S> {
    #[must_use]
    pub fn node(&self) -> &NodeId {
        &self.node
    }

    #[must_use]
    pub fn implementer(&self) -> &str {
        &self.implementer
    }

    /// The branch the node worked on, which is also its worktree's name.
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.branch
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
mod tests {
    use super::*;
    use crate::fanin::Claim;
    use kernel::{B3Hash, Locator};

    fn artifact(node: &str, by: &str, verifier: &str) -> Artifact {
        let digest = B3Hash::digest(b"work");
        Claim::new(
            NodeId::parse(node).unwrap(),
            Locator::parse(&format!("cas:b3-{digest}")).unwrap(),
            digest,
            by.to_owned(),
        )
        .verified(true, verifier)
        .unwrap()
    }

    fn request() -> Pr<Open> {
        Pr::open(
            NodeId::parse("node-1").unwrap(),
            "lab/room1".to_owned(),
            "node-1".to_owned(),
        )
        .unwrap()
    }

    #[test]
    fn verification_is_the_only_way_out_of_the_open_phase() {
        // `Pr<Open>` has no method that names a verifier; the phase that
        // carries one is the phase verification produces.
        let verified = request().verified(&artifact("node-1", "lab/room1", "lab/tests"));
        assert!(verified.is_ok());
        assert_eq!(verified.unwrap().verified_by(), "lab/tests");
    }

    #[test]
    fn the_implementer_cannot_be_the_verifier_at_either_gate() {
        // The first gate is in `fanin`: an artifact cannot be built by
        // its own producer. The second is here, in case a caller ever
        // hands over an artifact from somewhere else.
        let err = request()
            .verified(&artifact("node-1", "lab/tests", "lab/room1"))
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::EvidenceMissing);
        assert!(err.subject().contains("lab/room1"));
    }

    #[test]
    fn work_verified_for_another_node_does_not_merge_this_one() {
        let err = request()
            .verified(&artifact("node-2", "lab/room1", "lab/tests"))
            .unwrap_err();
        assert!(err.subject().contains("node-2"));
        assert!(err.recovery().contains("the node this request is for"));
    }

    #[test]
    fn the_request_carries_the_three_facts_a_record_is_built_from() {
        // Building the record is `pr_tool::request::OpenRequest`'s; this
        // phase only holds what it reads.
        let request = request();
        assert_eq!(request.node().as_str(), "node-1");
        assert_eq!(request.implementer(), "lab/room1");
        assert_eq!(request.branch(), "node-1");
    }
}
