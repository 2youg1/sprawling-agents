// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether one HTTP request may reach the work behind a door.
//!
//! The socket's peer is judged once, at the hello frame, and stays
//! judged for the life of the session; a POST has no session, so every
//! request is judged on its own. That is the whole difference between
//! this file and [`super::decide_handshake`], and it is why both of
//! them live in `reception` rather than in the shell that calls them.

use kernel::{AxCode, AxError};

use super::BindFace;
use super::pairing::Sessions;
use crate::auth::{self, Pairing};

/// Every credential this city accepts at this moment: the face's key,
/// and the browsers' live session tokens. One value, because the two are
/// one question (`crates/wire/spec/Reception/Admission.lean` §8-40).
#[derive(Debug, Clone, Copy)]
pub struct Keys<'a> {
    pub face: &'a BindFace,
    pub sessions: &'a Sessions,
}

impl Keys<'_> {
    /// Whether `offered` is this city's key or a live session token.
    #[must_use]
    pub fn pairing(&self, offered: Option<&str>) -> Pairing {
        let Some(offered) = offered else {
            return Pairing::Absent;
        };
        if auth::verify(Some(offered), self.face.key()) || self.sessions.holds(offered) {
            Pairing::Held
        } else {
            Pairing::Absent
        }
    }
}

/// Which HTTP door a request arrived at.
///
/// Exhaustive, because the pairing rule is not the same at all three:
/// a door that answers a stranger with a refusal and a door that lets
/// the refusal be worded further inward are different rules, and a
/// boolean would let a new door inherit whichever one was nearer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Door {
    /// `POST /transcribe`: bytes that cost money at a provider.
    Transcribe,
    /// `POST /enroll`: a credential on its way to the vault.
    Enroll,
    /// `POST /acp`: an outside editor driving the city.
    Acp,
    /// `POST /drop`: a file written onto the city's disk.
    Drop,
}

/// What a door does with one request, before any of its bytes are read.
#[derive(Debug)]
pub enum Admission {
    /// Let it through, carrying whether the caller held the token. Only
    /// [`Door::Acp`] is ever admitted unpaired.
    Admit(Pairing),
    Refuse(AxError),
}

/// Decides whether one HTTP request may reach the work behind a door,
/// once [`Keys::pairing`] has judged what it offered.
///
/// **The one place an HTTP caller's credential decides a door.** Every
/// face demands a key, so a caller with nothing is turned away from the
/// three doors that act wherever the city listens.
///
/// [`Door::Acp`] is admitted without one on purpose: what an
/// unauthenticated editor may learn is `agent_protocols::admit`'s to
/// word, and it words it so that a stranger learns exactly one bit.
#[must_use]
pub fn decide_admission(door: Door, pairing: Pairing) -> Admission {
    let action = match (pairing, door) {
        (Pairing::Held, _) => return Admission::Admit(Pairing::Held),
        (Pairing::Absent, Door::Acp) => return Admission::Admit(Pairing::Absent),
        (Pairing::Absent, Door::Transcribe) => "transcribe a recording",
        (Pairing::Absent, Door::Enroll) => "enrol a credential",
        (Pairing::Absent, Door::Drop) => "keep a dropped file",
    };
    Admission::Refuse(
        AxError::failure(
            AxCode::GateDenied,
            action,
            "the request carried neither this city's key nor a live session token",
        )
        .with_recovery(
            "send the session token or the city's key as an `Authorization: Bearer` header;              a program on this machine reads the key from the city's key file",
        ),
    )
}

/// The one spelling of how an HTTP caller offers the pairing token.
///
/// The socket offers it inside the hello frame, where the frame type
/// names the field; a POST has no frame, so it carries the standard
/// bearer header and this function is where that scheme is read. A
/// value that is not a bearer token is no token at all rather than a
/// token that happens to fail, so a caller cannot learn the scheme by
/// watching which refusal it gets.
#[must_use]
pub fn offered_pairing(header: Option<&str>) -> Option<&str> {
    let raw = header?.trim();
    let (scheme, token) = raw.split_once(' ')?;
    scheme
        .eq_ignore_ascii_case("Bearer")
        .then(|| token.trim())
        .filter(|token| !token.is_empty())
}

#[cfg(test)]
#[allow(clippy::panic, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn the_acting_doors_refuse_an_empty_hand_and_the_editor_door_carries_the_bit() {
        for door in [Door::Transcribe, Door::Enroll, Door::Drop] {
            let Admission::Refuse(err) = decide_admission(door, Pairing::Absent) else {
                panic!("{door:?} acts on a request, so it refuses an unpaired one");
            };
            assert_eq!(*err.code(), AxCode::GateDenied);
        }
        assert!(matches!(
            decide_admission(Door::Acp, Pairing::Absent),
            Admission::Admit(Pairing::Absent)
        ));
    }

    #[test]
    fn the_key_and_nothing_else_is_held_on_a_loopback_face() {
        let face = BindFace::Loopback {
            key: kernel::B3Hash::digest(b"native-key"),
        };
        let sessions = Sessions::default();
        let keys = Keys {
            face: &face,
            sessions: &sessions,
        };
        assert_eq!(keys.pairing(Some("native-key")), Pairing::Held);
        assert_eq!(keys.pairing(Some("guess")), Pairing::Absent);
        assert_eq!(keys.pairing(None), Pairing::Absent);
    }

    #[test]
    fn only_a_bearer_header_offers_a_token() {
        assert_eq!(offered_pairing(Some("Bearer abc")), Some("abc"));
        assert_eq!(offered_pairing(Some("bearer  abc ")), Some("abc"));
        assert_eq!(offered_pairing(Some("Basic abc")), None);
        assert_eq!(offered_pairing(Some("abc")), None);
        assert_eq!(offered_pairing(Some("Bearer ")), None);
        assert_eq!(offered_pairing(None), None);
    }
}
