// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, B3Hash, Locator};

use crate::workshop::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    node: NodeId,
    at: Locator,
    digest: B3Hash,
    by: String,
}

impl Claim {
    #[must_use]
    pub fn new(node: NodeId, at: Locator, digest: B3Hash, by: String) -> Claim {
        Claim {
            node,
            at,
            digest,
            by,
        }
    }

    #[must_use]
    pub fn node(&self) -> &NodeId {
        &self.node
    }

    #[must_use]
    pub fn at(&self) -> &Locator {
        &self.at
    }

    #[must_use]
    pub fn by(&self) -> &str {
        &self.by
    }

    pub fn verified(self, done_check_passed: bool, verifier: &str) -> Result<Artifact, AxError> {
        if verifier == self.by {
            return Err(AxError::failure(
                AxCode::EvidenceMissing,
                "verify a node result",
                format!("{} verified by its own producer", self.node.as_str()),
            )
            .with_recovery(
                "have another resident run the done check; a producer's own verdict on its \
                 own work is not verification",
            ));
        }
        if !done_check_passed {
            return Err(AxError::failure(
                AxCode::EvidenceMissing,
                "verify a node result",
                format!("{}: the done check did not pass", self.node.as_str()),
            )
            .with_recovery("fix the result until the node's own done check passes, then verify"));
        }
        Ok(Artifact {
            claim: self,
            verified_by: verifier.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    claim: Claim,
    verified_by: String,
}

impl Artifact {
    #[must_use]
    pub fn node(&self) -> &NodeId {
        self.claim.node()
    }

    #[must_use]
    pub fn at(&self) -> &Locator {
        self.claim.at()
    }

    #[must_use]
    pub fn by(&self) -> &str {
        self.claim.by()
    }

    #[must_use]
    pub fn verified_by(&self) -> &str {
        &self.verified_by
    }

    fn holds(&self, content: &str) -> bool {
        B3Hash::digest(content.as_bytes()) == self.claim.digest
            || B3Hash::digest(format!("{content}\n").as_bytes()) == self.claim.digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateQuestion {
    node: NodeId,
    prompt: String,
}

impl PrivateQuestion {
    #[must_use]
    pub fn node(&self) -> &NodeId {
        &self.node
    }

    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

#[derive(Debug, Default)]
pub struct FanIn {
    artifacts: BTreeMap<NodeId, Artifact>,
}

impl FanIn {
    #[must_use]
    pub fn new() -> FanIn {
        FanIn::default()
    }

    pub fn accept(&mut self, artifact: Artifact) {
        self.artifacts.insert(artifact.node().clone(), artifact);
    }

    pub fn artifacts(&self) -> impl Iterator<Item = &Artifact> {
        self.artifacts.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    pub fn question(&self) -> Result<PrivateQuestion, AxError> {
        let (node, artifact) = self.artifacts.first_key_value().ok_or_else(|| {
            AxError::failure(
                AxCode::EvidenceMissing,
                "pose the fan-in question",
                "no artifacts".to_owned(),
            )
            .with_recovery("verify at least one node's result before judging the join")
        })?;
        Ok(PrivateQuestion {
            node: node.clone(),
            prompt: format!(
                "before judging: open node {}'s artifact at {} and give its whole content, \
                 exactly as stored",
                node.as_str(),
                artifact.at()
            ),
        })
    }

    pub fn decide(&self, answer: &str) -> Result<Joined, AxError> {
        let question = self.question()?;
        let read = self
            .artifacts
            .get(&question.node)
            .is_some_and(|artifact| artifact.holds(answer));
        if !read {
            return Err(AxError::failure(
                AxCode::EvidenceMissing,
                "judge a fan-in",
                format!(
                    "the answer for node {} does not match",
                    question.node.as_str()
                ),
            )
            .with_recovery(
                "open the artifact and read it; the question is answerable only from its content",
            ));
        }
        Ok(Joined {
            nodes: self.artifacts.keys().cloned().collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joined {
    nodes: Vec<NodeId>,
}

impl Joined {
    #[must_use]
    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
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

    fn claim(node: &str, by: &str, content: &[u8]) -> Claim {
        let digest = B3Hash::digest(content);
        Claim::new(
            NodeId::parse(node).unwrap(),
            Locator::parse(&format!("cas:b3-{digest}")).unwrap(),
            digest,
            by.to_owned(),
        )
    }

    #[test]
    fn the_implementer_does_not_verify_their_own_work() {
        let err = claim("a", "lab/room1", b"result")
            .verified(true, "lab/room1")
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::EvidenceMissing);
        assert!(err.recovery().contains("another resident"));

        claim("a", "lab/room1", b"result")
            .verified(true, "lab/tests")
            .expect("a second resident may verify");
    }

    #[test]
    fn a_result_whose_done_check_failed_is_not_an_artifact() {
        let err = claim("a", "lab/room1", b"result")
            .verified(false, "lab/tests")
            .unwrap_err();
        assert!(err.subject().contains("done check"));
    }

    #[test]
    fn a_judge_that_did_not_open_the_work_does_not_get_to_judge() {
        let mut join = FanIn::new();
        join.accept(
            claim("a", "lab/room1", b"result")
                .verified(true, "lab/tests")
                .unwrap(),
        );

        let err = join.decide("i had a look").unwrap_err();
        assert_eq!(err.code(), &AxCode::EvidenceMissing);
        assert!(
            !err.to_string().contains("result"),
            "a refusal that leaks the answer teaches the shortcut"
        );
        let joined = join.decide("result").unwrap();
        assert_eq!(joined.nodes().len(), 1);
    }

    #[test]
    fn a_judge_that_copies_the_question_does_not_get_to_judge() {
        let mut join = FanIn::new();
        join.accept(
            claim("a", "lab/room1", b"result")
                .verified(true, "lab/tests")
                .unwrap(),
        );
        let prompt = join.question().unwrap().prompt().to_owned();
        let copied: String = prompt
            .split("cas:b3-")
            .nth(1)
            .unwrap()
            .chars()
            .take(8)
            .collect();
        assert!(
            join.decide(&copied).is_err(),
            "the question carries its own answer: {prompt}"
        );
    }

    #[test]
    fn the_question_follows_what_actually_joined() {
        let mut join = FanIn::new();
        assert!(join.question().is_err(), "nothing to have read yet");

        join.accept(
            claim("b", "lab/room1", b"second")
                .verified(true, "lab/tests")
                .unwrap(),
        );
        assert_eq!(join.question().unwrap().node().as_str(), "b");

        join.accept(
            claim("a", "lab/room2", b"first")
                .verified(true, "lab/tests")
                .unwrap(),
        );
        assert_eq!(
            join.question().unwrap().node().as_str(),
            "a",
            "the question is a function of what is in hand, not of the order it arrived"
        );
        assert!(join.question().unwrap().prompt().contains("whole content"));
    }
}
