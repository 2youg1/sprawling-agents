// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use remote_access::door::{Authority, SessionId, VerbClass, permits};
use remote_access::handshake::Session;
use remote_access::seal::{Opener, Payload, Sealer};

use super::keeper::Doorway;
use super::verbs::{Passage, passage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Step {
    Forward(String),

    Answer(Vec<u8>),

    Lock,
}

pub(super) struct Conduit {
    doorway: Doorway,
    session: SessionId,
    opener: Opener,
    sealer: Sealer,

    token: String,
}

impl Conduit {
    pub(super) fn new(doorway: Doorway, session: (SessionId, Session), token: String) -> Conduit {
        let (id, Session { sealer, opener }) = session;
        Conduit {
            doorway,
            session: id,
            opener,
            sealer,
            token,
        }
    }

    pub(super) fn judge(&mut self, sealed: &[u8]) -> Result<Step, AxError> {
        let authority = self.doorway.authority(self.session)?.ok_or_else(ended)?;
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
                if permits(authority, class) {
                    Ok(Step::Forward(text))
                } else {
                    self.answer(refused(authority, class))
                }
            }
        }
    }

    pub(super) fn seal_for_device(&mut self, text: &str) -> Result<Vec<u8>, AxError> {
        self.doorway.authority(self.session)?.ok_or_else(ended)?;
        self.sealer
            .seal(&Payload::Frame(text.to_owned()).to_bytes())
    }

    fn greeting(&self, said: wire::Hello) -> Result<Step, AxError> {
        let city = wire::ClientFrame::Hello(wire::Hello {
            token: Some(self.token.clone()),
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

impl Drop for Conduit {
    fn drop(&mut self) {
        if let Err(refused) = self.doorway.end_session(self.session) {
            eprintln!("  a remote session could not be released: {refused}");
        }
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
