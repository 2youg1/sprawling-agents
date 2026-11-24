// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, TimeMs};

use super::BindFace;
use super::sessions::Sessions;
use crate::auth::{self, Pairing};

#[derive(Debug, Clone, Copy)]
pub struct Keys<'a> {
    pub face: &'a BindFace,
    pub sessions: &'a Sessions,
    pub now: TimeMs,
}

impl Keys<'_> {
    #[must_use]
    pub fn pairing(&self, offered: Option<&str>) -> Pairing {
        let Some(offered) = offered else {
            return Pairing::Absent;
        };
        if auth::verify(Some(offered), self.face.key()) || self.sessions.holds(offered, self.now) {
            Pairing::Held
        } else {
            Pairing::Absent
        }
    }
}

#[derive(Debug)]
pub enum Standing {
    Stands,
    Ends(Box<AxError>),
}

#[must_use]
pub fn decide_standing(pairing: Pairing) -> Standing {
    match pairing {
        Pairing::Held => Standing::Stands,
        Pairing::Absent => Standing::Ends(Box::new(
            AxError::failure(
                AxCode::GateDenied,
                "keep a browser session open",
                "this browser was forgotten, or its session token has ended",
            )
            .with_recovery(
                "pair this browser again with the pairing code the terminal shows; a browser \
                 the city still knows reconnects by itself",
            ),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Door {
    Transcribe,

    Enroll,

    Acp,

    Drop,

    Workspace,
}

#[derive(Debug)]
pub enum Admission {
    Admit(Pairing),
    Refuse(AxError),
}

#[must_use]
pub fn decide_admission(door: Door, pairing: Pairing) -> Admission {
    let action = match (pairing, door) {
        (Pairing::Held, _) => return Admission::Admit(Pairing::Held),
        (Pairing::Absent, Door::Acp) => return Admission::Admit(Pairing::Absent),
        (Pairing::Absent, Door::Transcribe) => "transcribe a recording",
        (Pairing::Absent, Door::Enroll) => "enrol a credential",
        (Pairing::Absent, Door::Drop) => "keep a dropped file",
        (Pairing::Absent, Door::Workspace) => "open local workspace files",
    };
    Admission::Refuse(
        AxError::failure(
            AxCode::GateDenied,
            action,
            "the request carried neither this city's key nor a live session token",
        )
        .with_recovery(
            "send the session token or the city's key as an `Authorization: Bearer` header; \
             a program on this machine reads the key from the city's key file",
        ),
    )
}

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
        for door in [Door::Transcribe, Door::Enroll, Door::Drop, Door::Workspace] {
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
            now: kernel::TimeMs::new(0),
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
