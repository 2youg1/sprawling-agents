// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod admission;
mod entry;
pub(crate) mod inbound;
mod pairing;
mod sessions;

pub use admission::{
    Admission, Door, Keys, Standing, decide_admission, decide_standing, offered_pairing,
};
pub use entry::{Arrival, Entry, ListenerOrigins, PageHeaders, Presented, decide_entry};
pub use pairing::{
    BrowserDoor, DeviceKey, Guess, LABEL_MAX, OPEN_CODE_LIFETIME_MS, PairedBrowser, decode_hex,
};
pub use sessions::Sessions;

use std::net::SocketAddr;

use kernel::{Address, AxCode, AxError, B3Hash, Seq};

use crate::auth;
use crate::command::WireCommand;
use crate::frames::{
    Ask, ClientFrame, Hello, Lagged, Monitoring, WIRE_V, Watched, Welcome, schema_hash,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindFace {
    Loopback { key: B3Hash },

    Exposed { key: B3Hash },
}

impl BindFace {
    #[must_use]
    pub fn key(&self) -> &B3Hash {
        match self {
            BindFace::Loopback { key } | BindFace::Exposed { key } => key,
        }
    }
}

#[derive(Debug)]
pub enum BindVerdict {
    Serve(BindFace),
    Refuse(AxError),
}

#[must_use]
pub fn decide_bind(addr: &SocketAddr, key: Option<B3Hash>) -> BindVerdict {
    match (key, addr.ip().is_loopback()) {
        (Some(key), true) => BindVerdict::Serve(BindFace::Loopback { key }),
        (Some(key), false) => BindVerdict::Serve(BindFace::Exposed { key }),
        (None, _) => BindVerdict::Refuse(
            AxError::failure(
                AxCode::ConfigInvalid,
                "bind the control surface",
                format!("{addr} was given no key to demand of its callers"),
            )
            .with_recovery("configure a pairing token, or let the host mint a key for this serve"),
        ),
    }
}

#[derive(Debug)]
pub enum EnrollVerdict {
    Accept,
    Refuse(AxError),
}

#[must_use]
pub fn decide_enroll(peer: &SocketAddr) -> EnrollVerdict {
    if peer.ip().to_canonical().is_loopback() {
        return EnrollVerdict::Accept;
    }
    EnrollVerdict::Refuse(
        AxError::failure(
            AxCode::GateDenied,
            "enrol a credential",
            format!("{peer} is not on this machine"),
        )
        .with_recovery(
            "enrol the credential from the machine running sprawling; a tunnelled session can \
             use it afterwards but cannot deliver it",
        ),
    )
}

#[derive(Debug)]
pub enum HandshakeVerdict {
    Accept,
    Reject(AxError),
}

#[must_use]
pub fn decide_handshake(hello: &Hello, expected: &Welcome, keys: &Keys<'_>) -> HandshakeVerdict {
    if hello.wire_v != expected.wire_v || hello.schema != expected.schema {
        return HandshakeVerdict::Reject(
            AxError::failure(
                AxCode::WireMismatch,
                "accept a client connection",
                format!(
                    "client speaks wire v{} and this server speaks v{}",
                    hello.wire_v, expected.wire_v
                ),
            )
            .with_recovery("reload the page to fetch the client this server was built with"),
        );
    }
    match keys.pairing(hello.token.as_deref()) {
        auth::Pairing::Held => HandshakeVerdict::Accept,
        auth::Pairing::Absent => HandshakeVerdict::Reject(
            AxError::failure(
                AxCode::ConfigInvalid,
                "accept a client connection",
                "the greeting carries neither this city's key nor a live session token",
            )
            .with_recovery(
                "reload the page to sign in again; a program on this machine reads the key \
                 from the city's key file, or takes --token",
            ),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    AwaitingHello,
    Live,
}

#[derive(Debug)]
pub enum SessionStep {
    Welcome(Box<Welcome>),

    Deliver(Box<WireCommand>),

    Answer(Box<Ask>),

    Watch(Watched),

    Release,

    Beat(crate::BeatMs),

    Refuse { error: Box<AxError>, close: bool },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WelcomeFacts<'a> {
    pub city: Option<&'a Address>,
    pub head: Option<Seq>,
    pub epoch: Option<B3Hash>,
}

#[must_use]
pub fn decide_frame(
    state: SessionState,
    frame: ClientFrame,
    keys: &Keys<'_>,
    standing: WelcomeFacts<'_>,
) -> SessionStep {
    let expected = Welcome {
        wire_v: WIRE_V,
        schema: schema_hash(),
        resume_from: standing.head,
        city: standing.city.cloned(),
        epoch: standing.epoch,
    };
    match (state, frame) {
        (SessionState::AwaitingHello, ClientFrame::Hello(hello)) => {
            match decide_handshake(&hello, &expected, keys) {
                HandshakeVerdict::Accept => SessionStep::Welcome(Box::new(expected)),
                HandshakeVerdict::Reject(error) => SessionStep::Refuse {
                    error: Box::new(error),
                    close: true,
                },
            }
        }
        (SessionState::AwaitingHello, _) => SessionStep::Refuse {
            error: Box::new(
                AxError::failure(
                    AxCode::WireMismatch,
                    "accept a client frame",
                    "the session has not been opened with a hello",
                )
                .with_recovery("send hello first; reload the page if the client did not"),
            ),
            close: true,
        },
        (SessionState::Live, ClientFrame::Command(command)) => SessionStep::Deliver(command),
        (SessionState::Live, ClientFrame::Ask(ask)) => SessionStep::Answer(Box::new(ask)),
        (SessionState::Live, ClientFrame::Monitor(Monitoring::Watch)) => {
            SessionStep::Watch(Watched::Everything)
        }
        (SessionState::Live, ClientFrame::Monitor(Monitoring::WatchSummary)) => {
            SessionStep::Watch(Watched::Summary)
        }
        (SessionState::Live, ClientFrame::Monitor(Monitoring::Release)) => SessionStep::Release,
        (SessionState::Live, ClientFrame::Monitor(Monitoring::Beat(beat))) => {
            SessionStep::Beat(beat)
        }
        (SessionState::Live, ClientFrame::Hello(_)) => SessionStep::Refuse {
            error: Box::new(
                AxError::failure(
                    AxCode::WireMismatch,
                    "accept a client frame",
                    "this session is already open",
                )
                .with_recovery("open a second connection instead of re-greeting on this one"),
            ),
            close: false,
        },
    }
}

#[must_use]
pub(crate) fn decide_lag(delivered: Option<Seq>, next: Seq) -> Option<Lagged> {
    let from = match delivered {
        Some(last) => last.value().checked_add(1).map(Seq::new),

        None => Some(Seq::FIRST),
    };
    let to = next.value().checked_sub(1).map(Seq::new);
    match (from, to) {
        (Some(from), Some(to)) if to >= from => Some(Lagged { from, to }),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    Even(Option<Seq>),

    Owed(Option<Seq>),
}

impl Stream {
    pub(crate) const fn opening() -> Stream {
        Stream::Even(None)
    }

    #[must_use]
    pub(crate) fn delivered(self) -> Option<Seq> {
        match self {
            Stream::Even(delivered) | Stream::Owed(delivered) => delivered,
        }
    }

    #[must_use]
    pub(crate) fn skipped(self, live: bool) -> Stream {
        if live {
            Stream::Owed(self.delivered())
        } else {
            self
        }
    }

    #[must_use]
    pub(crate) fn before(self, next: Seq) -> (Option<Lagged>, Stream) {
        let lag = match self {
            Stream::Even(None) => None,
            Stream::Even(delivered @ Some(_)) | Stream::Owed(delivered) => {
                decide_lag(delivered, next)
            }
        };
        (lag, Stream::Even(Some(next)))
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
mod tests;
