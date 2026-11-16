// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

use crate::fanin::Artifact;
use crate::workshop::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pr<S> {
    node: NodeId,
    implementer: String,
    branch: String,
    state: S,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Open;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    by: String,
}

impl Pr<Open> {
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
        let verified = request().verified(&artifact("node-1", "lab/room1", "lab/tests"));
        assert!(verified.is_ok());
        assert_eq!(verified.unwrap().verified_by(), "lab/tests");
    }

    #[test]
    fn the_implementer_cannot_be_the_verifier_at_either_gate() {
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
        let request = request();
        assert_eq!(request.node().as_str(), "node-1");
        assert_eq!(request.implementer(), "lab/room1");
        assert_eq!(request.branch(), "node-1");
    }
}
