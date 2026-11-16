// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, Locator, Payload, TimeMs, Version};
use serde::{Deserialize, Serialize};

use crate::fanin::{Artifact, Claim};
use crate::inbox::Signal;
use crate::workshop::NodeId;
use kernel::event::record::{SignalId, SignalKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handback {

    Finished(Artifact),

    Stopped { claim: Claim, because: String },
}

impl Handback {

    #[must_use]
    pub fn of(claim: Claim, done_check_passed: bool, verifier: &str) -> Handback {
        match claim.clone().verified(done_check_passed, verifier) {
            Ok(artifact) => Handback::Finished(artifact),
            Err(refusal) => Handback::Stopped {
                claim,
                because: refusal.to_string(),
            },
        }
    }

    #[must_use]
    pub fn unstarted(claim: Claim, refusal: &AxError) -> Handback {
        Handback::Stopped {
            claim,
            because: format!("it could not start: {refusal}; {}", refusal.recovery()),
        }
    }

    #[must_use]
    pub fn node(&self) -> &NodeId {
        match self {
            Handback::Finished(artifact) => artifact.node(),
            Handback::Stopped { claim, .. } => claim.node(),
        }
    }

    #[must_use]
    pub fn by(&self) -> &str {
        match self {
            Handback::Finished(artifact) => artifact.by(),
            Handback::Stopped { claim, .. } => claim.by(),
        }
    }

    pub fn signal(&self, id: SignalId, to: Address, at: TimeMs) -> Result<Signal, AxError> {
        let body = match self {
            Handback::Finished(artifact) => HandbackBody::Finished {
                room: artifact.node().clone(),
                at: artifact.at().clone(),
                verified_by: artifact.verified_by().to_owned(),
                text: format!(
                    "{} finished the work you handed down; its account is pinned at {}",
                    self.by(),
                    artifact.at()
                ),
            },
            Handback::Stopped { claim, because } => HandbackBody::Stopped {
                room: claim.node().clone(),
                at: claim.at().clone(),
                because: because.clone(),
                text: format!(
                    "{} did not finish the work you handed down: {because}",
                    self.by()
                ),
            },
        };
        Signal::new(
            id,
            SignalKind::Thread,
            self.by().to_owned(),
            to,
            Version::FIRST,
            Payload::of(&body)?,
            at,
        )
    }

    pub fn from_signal(signal: &Signal) -> Result<Option<Handback>, AxError> {
        if !signal.payload().as_map().contains_key(HandbackBody::TAG) {
            return Ok(None);
        }
        let body: HandbackBody = signal.payload().read()?;
        let (room, at) = match &body {
            HandbackBody::Finished { room, at, .. } | HandbackBody::Stopped { room, at, .. } => {
                (room.clone(), at.clone())
            }
        };
        let Locator::Cas { hash, .. } = at else {
            return Err(AxError::failure(
                AxCode::WireMismatch,
                "read a handback",
                at.to_string(),
            )
            .with_recovery("a handback's account is a `cas:b3-…` locator; replay with the build that wrote this line"));
        };
        let claim = Claim::new(room, at.clone(), hash, signal.from().to_owned());
        match body {
            HandbackBody::Finished { verified_by, .. } => Ok(Some(Handback::Finished(
                claim.verified(true, &verified_by)?,
            ))),
            HandbackBody::Stopped { because, .. } => Ok(Some(Handback::Stopped { claim, because })),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "handback", rename_all = "snake_case")]
enum HandbackBody {
    Finished {
        room: NodeId,
        at: Locator,
        verified_by: String,
        text: String,
    },
    Stopped {
        room: NodeId,
        at: Locator,
        because: String,
        text: String,
    },
}

impl HandbackBody {

    const TAG: &'static str = "handback";
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
    use kernel::{B3Hash, Locator};

    fn claim(by: &str) -> Claim {
        let digest = B3Hash::digest(b"the account of a delegate");
        Claim::new(
            NodeId::parse("lab/helper").unwrap(),
            Locator::parse(&format!("cas:b3-{digest}")).unwrap(),
            digest,
            by.to_owned(),
        )
    }

    #[test]
    fn a_delegate_that_finished_comes_back_as_a_verified_artifact() {
        let back = Handback::of(claim("lab/helper"), true, "city");
        assert!(matches!(back, Handback::Finished(_)));
        assert_eq!(back.by(), "lab/helper");
        assert_eq!(back.node().as_str(), "lab/helper");

        let signal = back
            .signal(
                SignalId::parse("handback-1").unwrap(),
                Address::parse("lab/room1").unwrap(),
                TimeMs::new(90),
            )
            .unwrap();
        assert_eq!(signal.from(), "lab/helper");
        assert_eq!(signal.room().as_str(), "lab/room1");
        let body = signal.payload().as_map();
        assert_eq!(body["handback"], "finished");
        assert_eq!(body["verified_by"], "city");
        assert!(body["text"].as_str().unwrap().contains("finished"));
    }

    #[test]
    fn a_delegate_that_stopped_still_reaches_the_run_that_asked() {
        let back = Handback::of(claim("lab/helper"), false, "city");
        let Handback::Stopped { ref because, .. } = back else {
            panic!("a failed done check does not verify");
        };
        assert!(because.contains("done check"));

        let signal = back
            .signal(
                SignalId::parse("handback-2").unwrap(),
                Address::parse("lab/room1").unwrap(),
                TimeMs::new(91),
            )
            .unwrap();
        let body = signal.payload().as_map();
        assert_eq!(body["handback"], "stopped");
        assert!(body["text"].as_str().unwrap().contains("did not finish"));
        assert!(body.contains_key("because"));
    }

    #[test]
    fn the_signal_reads_back_as_the_handback_that_wrote_it() {
        let finished = Handback::of(claim("lab/helper"), true, "city");
        let signal = finished
            .signal(
                SignalId::parse("handback-3").unwrap(),
                Address::parse("lab/room1").unwrap(),
                TimeMs::new(92),
            )
            .unwrap();
        assert_eq!(Handback::from_signal(&signal).unwrap(), Some(finished));

        let stopped = Handback::of(claim("lab/helper"), false, "city");
        let signal = stopped
            .signal(
                SignalId::parse("handback-4").unwrap(),
                Address::parse("lab/room1").unwrap(),
                TimeMs::new(93),
            )
            .unwrap();
        let Some(Handback::Stopped { because, .. }) = Handback::from_signal(&signal).unwrap()
        else {
            panic!("a stop is not a silence");
        };
        assert!(because.contains("done check"));

        let ordinary = Signal::new(
            SignalId::parse("s-1").unwrap(),
            SignalKind::Mention,
            "lab/room2".to_owned(),
            Address::parse("lab/room1").unwrap(),
            Version::FIRST,
            Payload::empty(),
            TimeMs::new(94),
        )
        .unwrap();
        assert_eq!(Handback::from_signal(&ordinary).unwrap(), None);
    }

    #[test]
    fn a_child_that_could_not_start_comes_back_stopped_with_the_refusal() {
        let refusal = AxError::failure(
            AxCode::ModelUnchosen,
            "choose the main model",
            "no model is chosen for this tag",
        )
        .with_recovery("pick a model for this tag");
        let back = Handback::unstarted(claim("lab/helper"), &refusal);
        let signal = back
            .signal(
                SignalId::parse("handback-5").unwrap(),
                Address::parse("lab/room1").unwrap(),
                TimeMs::new(95),
            )
            .unwrap();
        let text = signal.payload().as_map()["text"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(text.contains("E_MODEL_UNCHOSEN"), "{text}");
        assert!(text.contains("pick a model for this tag"), "{text}");
        assert_eq!(Handback::from_signal(&signal).unwrap(), Some(back));
    }

    #[test]
    fn the_child_is_not_allowed_to_be_its_own_verifier() {
        let back = Handback::of(claim("lab/helper"), true, "lab/helper");
        assert!(matches!(back, Handback::Stopped { .. }));
    }
}
