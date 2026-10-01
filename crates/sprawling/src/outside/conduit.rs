// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One remote session's frames, judged one at a time
//! (sprawling-SPEC.md 8-139; crates/remote_access/Spec.lean §8-5, §8-10).
//!
//! A sealed payload from the device is opened, read by its first byte,
//! and, when it is a wire frame, classed (`outside::verbs`) and judged by
//! the door against the session's authority at this moment. A frame the
//! door permits goes on to the city exactly as the device wrote it; a
//! frame it refuses goes nowhere, and the device is answered with a
//! sealed `Refusal` frame instead. The city's own frames come back
//! sealed. No socket is touched here, so a test drives the same steps a
//! connection does.
//!
//! The point a reader most often gets wrong: a session the door no
//! longer holds is not refused frame by frame, it ends. The door closed,
//! the device was revoked or the time ran out, and every later frame
//! would be refused for the same reason.

use kernel::{AxCode, AxError};
use remote_access::door::{Authority, SessionId, VerbClass, permits};
use remote_access::handshake::Session;
use remote_access::seal::{Opener, Payload, Sealer};

use super::keeper::Doorway;
use super::verbs::{Passage, passage};

/// What one frame from the device comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Step {
    /// Text for the city's `/ws`, as the device wrote it unless it was
    /// the greeting.
    Forward(String),
    /// A sealed answer for the device; nothing reaches the city.
    Answer(Vec<u8>),
    /// The device locked the door behind it.
    Lock,
}

pub(super) struct Conduit {
    doorway: Doorway,
    session: SessionId,
    opener: Opener,
    sealer: Sealer,
    /// The city's own pairing token, which the greeting carries in.
    token: Option<String>,
}

impl Conduit {
    pub(super) fn new(
        doorway: Doorway,
        session: (SessionId, Session),
        token: Option<String>,
    ) -> Conduit {
        let (id, Session { sealer, opener }) = session;
        Conduit {
            doorway,
            session: id,
            opener,
            sealer,
            token,
        }
    }

    /// Opens and judges one sealed payload from the device.
    ///
    /// # Errors
    /// A payload that does not open or read, and a session the door no
    /// longer holds: either ends the connection.
    pub(super) fn judge(&mut self, sealed: &[u8]) -> Result<Step, AxError> {
        let text = match Payload::from_bytes(&self.opener.open(sealed)?)? {
            Payload::Lock => return Ok(Step::Lock),
            Payload::Frame(text) => text,
        };
        let frame: wire::ClientFrame = match serde_json::from_str(&text) {
            Ok(frame) => frame,
            Err(unread) => {
                let refusal = AxError::failure(
                    AxCode::WireMismatch,
                    "carry a frame from a remote device",
                    unread.to_string(),
                )
                .with_recovery("reload the page: it speaks another version of the wire");
                return self.answer(refusal);
            }
        };
        match passage(frame) {
            Passage::Greeting(said) => self.greeting(said),
            Passage::Judged(class) => {
                let authority = self.doorway.authority(self.session)?.ok_or_else(ended)?;
                if permits(authority, class) {
                    Ok(Step::Forward(text))
                } else {
                    self.answer(refused(authority, class))
                }
            }
        }
    }

    /// Seals one frame the city sent, for the device.
    ///
    /// # Errors
    /// The session's counter is spent.
    pub(super) fn seal_for_device(&mut self, text: &str) -> Result<Vec<u8>, AxError> {
        self.sealer
            .seal(&Payload::Frame(text.to_owned()).to_bytes())
    }

    /// The device's greeting, sent on with the city's own token: the
    /// version and schema stay the device's, so the city still judges
    /// whether the page speaks its wire.
    fn greeting(&self, said: wire::Hello) -> Result<Step, AxError> {
        let city = wire::ClientFrame::Hello(wire::Hello {
            token: self.token.clone(),
            ..said
        });
        serde_json::to_string(&city)
            .map(Step::Forward)
            .map_err(|unwritten| {
                AxError::failure(
                    AxCode::WireMismatch,
                    "greet the city for a remote device",
                    unwritten.to_string(),
                )
                .with_recovery("report this: a greeting this build read it could not write")
            })
    }

    fn answer(&mut self, refusal: AxError) -> Result<Step, AxError> {
        let frame = wire::ServerFrame::Refusal(Box::new(refusal));
        let text = serde_json::to_string(&frame).map_err(|unwritten| {
            AxError::failure(
                AxCode::WireMismatch,
                "answer a remote device",
                unwritten.to_string(),
            )
            .with_recovery("report this: a refusal this build made it could not write")
        })?;
        self.seal_for_device(&text).map(Step::Answer)
    }
}

fn refused(authority: Authority, class: VerbClass) -> AxError {
    match (authority, class) {
        (Authority::Watch, VerbClass::Act) => AxError::failure(
            AxCode::GateDenied,
            "carry a verb from a remote device",
            "this device was paired to watch",
        )
        .with_recovery("pair the device again without `--watch` from the city's console"),
        (Authority::Watch | Authority::Act, VerbClass::LocalOnly | VerbClass::Read)
        | (Authority::Act, VerbClass::Act) => AxError::failure(
            AxCode::GateDenied,
            "carry a verb from a remote device",
            "this verb is only carried out at the city's own machine",
        )
        .with_recovery("do this at the machine the city runs on"),
    }
}

fn ended() -> AxError {
    AxError::failure(
        AxCode::GateDenied,
        "carry a frame from a remote device",
        "the remote session is over",
    )
    .with_recovery("connect again; if the door is closed, open it on the city's console")
}
