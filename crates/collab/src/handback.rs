// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run is told about work it handed down.
//!
//! **The way back is not the next turn.** Only the assembly layer can
//! build a run, and it gets control back when the parent is already
//! frozen, so a child starts after its parent's last turn rather than
//! between two of them. "The parent's next turn" is therefore the next
//! run in the parent's room, and the door that already carries a fact
//! across runs is that room's `Inbox`. A second door would be a second
//! answer to "what is waiting for this resident", so the way back is an
//! ordinary [`Signal`] and `status.signals_pending` counts it without
//! being taught anything new.
//!
//! The city, not the child, is the verifier. `Claim::verified` refuses a
//! producer judging its own work, and `Completion::Done(Evidence)` is
//! something the city observed rather than something the child said -
//! which is exactly what makes an [`Artifact`] constructible here.

use kernel::{Address, AxCode, AxError, Locator, Payload, TimeMs, Version};
use serde::{Deserialize, Serialize};

use crate::fanin::{Artifact, Claim};
use crate::inbox::{Signal, SignalId, SignalKind};
use crate::workshop::NodeId;

/// How one piece of handed-down work came back. Exhaustive: a delegate
/// that finished and one that stopped are different facts, and a parent
/// told only that something came back would have to guess which.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handback {
    /// The child's own done check passed, and somebody other than the
    /// child said so. The name avoids `Verified`, which this crate
    /// already uses for a pull request's phase.
    Finished(Artifact),
    /// It stopped without evidence, or its claim did not verify.
    /// `because` carries the refusal's own words rather than a summary
    /// of them.
    Stopped { claim: Claim, because: String },
}

impl Handback {
    /// Verifies the child's claim on the city's behalf.
    ///
    /// A refusal is an outcome here rather than a failure: a parent that
    /// is told nothing when its delegate stopped can only find out by
    /// waiting, which is the one thing an agent must never be asked to
    /// do.
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

    /// Which room the work was done in. The node id is that room's
    /// address: a delegate's identity is where it worked.
    #[must_use]
    pub fn node(&self) -> &NodeId {
        match self {
            Handback::Finished(artifact) => artifact.node(),
            Handback::Stopped { claim, .. } => claim.node(),
        }
    }

    /// Who did the work.
    #[must_use]
    pub fn by(&self) -> &str {
        match self {
            Handback::Finished(artifact) => artifact.by(),
            Handback::Stopped { claim, .. } => claim.by(),
        }
    }

    /// The signal the asking run's room receives.
    ///
    /// `Thread` rather than `Mention`: this answers something that room
    /// asked for, and the lane it takes is derived from that kind rather
    /// than chosen here.
    ///
    /// # Errors
    /// Propagates the payload's and the signal's own refusals.
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

    /// The one inverse of [`signal`](Self::signal).
    ///
    /// `None` is an ordinary signal between residents, which reports no
    /// handed-down work. A signal that says it is a handback and cannot
    /// be read as one is an error rather than a `None`: the reader that
    /// used to live in the fold answered `None` to both questions, so a
    /// delegate that stopped and a line this build cannot parse arrived
    /// at the parent as the same silence.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` for a handback body this build cannot read,
    /// an account that is not a CAS locator, and a claim the recorded
    /// verifier is not allowed to have verified.
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

/// The handback signal's payload: the one authority for its keys.
///
/// Internally tagged, so the line reads exactly as the hand-written
/// writer left it - `handback` beside the fields, not wrapping them -
/// and so the two outcomes cannot be confused for one shape with
/// optional halves.
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
    /// The key that says a signal is a handback at all, spelled here
    /// because the serde tag above and the probe in
    /// [`Handback::from_signal`] are the same key.
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

    /// The one the parent would otherwise never hear about.
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

    /// The reader the fold used to hold, now the writer's own inverse:
    /// a delegate that stopped comes back as a stop with its reason
    /// rather than as the same `None` an ordinary signal answers.
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

    /// The rule `Claim::verified` holds, reached through this door: the
    /// city may verify, the child may not verify itself.
    #[test]
    fn the_child_is_not_allowed_to_be_its_own_verifier() {
        let back = Handback::of(claim("lab/helper"), true, "lab/helper");
        assert!(matches!(back, Handback::Stopped { .. }));
    }
}
