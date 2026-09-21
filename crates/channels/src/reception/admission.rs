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

use kernel::{AxCode, AxError, B3Hash};

use crate::auth;

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
}

/// What a door does with one request, before any of its bytes are read.
#[derive(Debug)]
pub enum Admission {
    /// Let it through, carrying whether the caller held the token. Only
    /// [`Door::Acp`] is ever admitted unpaired.
    Admit(Pairing),
    Refuse(AxError),
}

/// Whether the caller held this city's pairing token.
///
/// An enum rather than a boolean so that neither a door nor an
/// admission can pass the verdict the wrong way round and still
/// compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pairing {
    Held,
    Absent,
}

/// Decides whether one HTTP request may reach the work behind a door.
///
/// **The one place an HTTP caller's pairing token is judged.** Before
/// it existed, `/ws` and `/acp` judged the token and `/transcribe` and
/// `/enroll` judged nothing, so an exposed city let anyone who could
/// reach the port spend money at a provider (roadmap S-03).
///
/// A city with no token configured is a city on loopback only, which
/// [`decide_bind`] guarantees at startup; there is nobody to
/// distinguish, so every door admits as paired.
///
/// [`Door::Acp`] is admitted unpaired on purpose: what an
/// unauthenticated editor may learn is `protocol::admit`'s to word, and
/// it words it so that a stranger learns exactly one bit. The other two
/// doors act, so they refuse here.
#[must_use]
pub fn decide_admission(
    door: Door,
    offered: Option<&str>,
    configured: Option<&B3Hash>,
) -> Admission {
    let Some(expected) = configured else {
        return Admission::Admit(Pairing::Held);
    };
    if auth::verify(offered, expected) {
        return Admission::Admit(Pairing::Held);
    }
    let action = match door {
        Door::Transcribe => "transcribe a recording",
        Door::Enroll => "enrol a credential",
        Door::Acp => return Admission::Admit(Pairing::Absent),
    };
    Admission::Refuse(
        AxError::failure(
            AxCode::GateDenied,
            action,
            "the request did not carry this city's pairing token",
        )
        .with_recovery(
            "open the control surface from the link that carries the pairing code, or send the \
             code as an `Authorization: Bearer` header",
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
    fn the_two_acting_doors_refuse_a_stranger_and_the_editor_door_carries_the_bit() {
        let digest = B3Hash::digest(b"pairing-code");
        for door in [Door::Transcribe, Door::Enroll] {
            let Admission::Refuse(err) = decide_admission(door, None, Some(&digest)) else {
                panic!("{door:?} acts on a request, so it refuses an unpaired one");
            };
            assert_eq!(*err.code(), AxCode::GateDenied);
        }
        assert!(matches!(
            decide_admission(Door::Acp, None, Some(&digest)),
            Admission::Admit(Pairing::Absent)
        ));
        assert!(matches!(
            decide_admission(Door::Transcribe, Some("pairing-code"), Some(&digest)),
            Admission::Admit(Pairing::Held)
        ));
    }

    #[test]
    fn a_city_with_no_token_configured_admits_every_door() {
        for door in [Door::Transcribe, Door::Enroll, Door::Acp] {
            assert!(
                matches!(
                    decide_admission(door, None, None),
                    Admission::Admit(Pairing::Held)
                ),
                "{door:?} on a loopback-only city"
            );
        }
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
