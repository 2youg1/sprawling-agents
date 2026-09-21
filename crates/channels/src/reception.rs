// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every judgement the process boundary makes, as pure functions with
//! exhaustive verdicts.
//!
//! Five questions, asked in five different places and answered here:
//! may this address be bound at all and which face does it then present,
//! may this peer put a credential in the vault, may this peer's opening
//! frame be accepted, what does a frame mean in the state the session is
//! in, and which records must a session tell the peer it lost. None of
//! them touches a socket, a clock or a file, so all five are tested by
//! calling them.
//!
//! **This is the thick half of the Humble Object** the listener is
//! built as (ARCHITECTURE section 9). `channels::server` is declared an
//! adapter — "thin, no policy" — and policy is what these functions
//! are; a shell that also holds the rules it applies has no way to show
//! that it applies all of them. Every branch left in the shell is a
//! send, a receive, or the end of a session.
//!
//! Binding is loopback by default, and the face that comes back from
//! [`decide_bind`] carries the credential it demands: a listener
//! reachable beyond this machine refuses to start without one, and no
//! shell can be built with an exposed face that demands nothing.

mod admission;
pub(crate) mod inbound;

pub use admission::{Admission, Door, Pairing, decide_admission, offered_pairing};

use std::net::SocketAddr;

use kernel::{Address, AxCode, AxError, B3Hash, Seq};

use crate::auth;
use crate::command::WireCommand;
use crate::wire::{ClientFrame, Hello, Lagged, Query, WIRE_V, Welcome, schema_hash};

/// Which face the listener presents, and the credential it demands.
///
/// **The digest travels inside the exposed face rather than beside it.**
/// A listener reachable beyond this machine with no credential is not a
/// state this type can hold, and [`decide_bind`] is the only producer of
/// one: it refuses that configuration before the socket exists. A
/// `token_configured: bool` argument said the same thing and left every
/// shell free to disagree with the verdict it was handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindFace {
    /// Reachable from this machine only. A token may still be configured - a
    /// person set one on a city they serve to nobody - and the doors then
    /// demand it exactly as the exposed face does.
    Loopback { token: Option<B3Hash> },
    /// Reachable from elsewhere, and never served without a credential.
    Exposed { token: B3Hash },
}

impl BindFace {
    /// The digest this face demands of every caller. `None` only on the
    /// loopback face of a city nobody configured a token for.
    #[must_use]
    pub fn token_digest(&self) -> Option<&B3Hash> {
        match self {
            BindFace::Loopback { token } => token.as_ref(),
            BindFace::Exposed { token } => Some(token),
        }
    }
}

/// The whole of the binding policy.
#[derive(Debug)]
pub enum BindVerdict {
    Serve(BindFace),
    Refuse(AxError),
}

/// Decides whether the listener may bind `addr`, and which face it then
/// presents.
///
/// Pure. Four cells, one of which refuses: an address reachable from outside
/// this machine with no pairing token configured. The refusal happens before
/// the socket exists, so there is no window in which the port is open and
/// unauthenticated - and the credential the exposed face demands comes back
/// inside the verdict, so there is no second way to learn it.
#[must_use]
pub fn decide_bind(addr: &SocketAddr, token: Option<B3Hash>) -> BindVerdict {
    if addr.ip().is_loopback() {
        return BindVerdict::Serve(BindFace::Loopback { token });
    }
    match token {
        Some(token) => BindVerdict::Serve(BindFace::Exposed { token }),
        None => BindVerdict::Refuse(
            AxError::failure(
                AxCode::ConfigInvalid,
                "bind the control surface",
                format!(
                    "{addr} is reachable beyond this machine and no pairing token is configured"
                ),
            )
            .with_recovery(
                "configure a pairing token before exposing the port, or bind a loopback address",
            ),
        ),
    }
}
/// Whether one peer may enrol a credential.
#[derive(Debug)]
pub enum EnrollVerdict {
    Accept,
    Refuse(AxError),
}

/// Decides whether `peer` may put plaintext into this machine's vault.
///
/// Pure, and the whole of the policy: only a caller on this machine may.
/// A pairing token is not enough, because a token authenticates a person
/// and this rule is about where the bytes travel. The design's guarantee
/// is that a credential cannot be enrolled remotely at all - the socket
/// half is type-level (`PutSecret` has no wire form), and this is the
/// HTTP half, which needs a runtime check because bytes can always be
/// posted at a route.
#[must_use]
pub fn decide_enroll(peer: &SocketAddr) -> EnrollVerdict {
    if peer.ip().is_loopback() {
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

/// The verdict on one peer's opening frame.
#[derive(Debug)]
pub enum HandshakeVerdict {
    Accept,
    Reject(AxError),
}

/// Decides whether to accept a peer.
///
/// Order is deliberate: protocol agreement is settled before credentials.
/// A browser holding a cached older client is the common case and deserves
/// "refresh", not "wrong password". Pure - `expected` and `face` are
/// parameters, never read from ambient state.
///
/// The credential a caller must present is the face's, which means an
/// exposed face cannot be served without one: [`BindFace::Exposed`] holds
/// the digest, and there is no other way to reach this function.
///
/// The digest is a digest, not the token. This crate never holds the
/// plaintext of a pairing token: the side that owns the token digests it
/// once, and the boundary compares digests. That keeps credential exposure
/// at the redemption points where it is audited, and it costs nothing here
/// because the comparison hashes both sides anyway.
#[must_use]
pub fn decide_handshake(hello: &Hello, expected: &Welcome, face: &BindFace) -> HandshakeVerdict {
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
    let Some(expected_digest) = face.token_digest() else {
        return HandshakeVerdict::Accept;
    };
    if auth::verify(hello.token.as_deref(), expected_digest) {
        return HandshakeVerdict::Accept;
    }
    HandshakeVerdict::Reject(
        AxError::failure(
            AxCode::ConfigInvalid,
            "accept a client connection",
            "the pairing token does not match",
        )
        .with_recovery("re-enter the pairing code shown on the host machine"),
    )
}
/// How far a socket session has got. Two states, because there are two:
/// a peer that has not identified itself, and one that has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    AwaitingHello,
    Live,
}

/// What the shell does with one client frame. Exhaustive: a new frame
/// kind has to be answered here rather than falling through to silence.
#[derive(Debug)]
pub enum SessionStep {
    /// Answer with this welcome and move to [`SessionState::Live`].
    Welcome(Box<Welcome>),
    /// Hand this command to the sink.
    Deliver(Box<WireCommand>),
    /// Evaluate this query and answer it.
    Answer(Box<Query>),
    /// Send this refusal; `close` ends the session afterwards.
    Refuse { error: Box<AxError>, close: bool },
}

/// The whole session policy, as a pure function: which frames are legal
/// when, and what a mismatch does. Tested without a socket, for the same
/// reason [`decide_bind`] is.
///
/// A command that arrives before the hello is refused rather than queued:
/// the peer has not yet shown that it speaks this wire, and running work
/// for it would be trusting a stranger's first sentence.
#[must_use]
pub fn decide_frame(
    state: SessionState,
    frame: ClientFrame,
    face: &BindFace,
    city: Option<&Address>,
) -> SessionStep {
    let expected = Welcome {
        wire_v: WIRE_V,
        schema: schema_hash(),
        resume_from: None,
        city: city.cloned(),
    };
    match (state, frame) {
        (SessionState::AwaitingHello, ClientFrame::Hello(hello)) => {
            match decide_handshake(&hello, &expected, face) {
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
        (SessionState::Live, ClientFrame::Query(query)) => SessionStep::Answer(Box::new(query)),
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

/// The range of ledger records a session must name when its event stream
/// skipped some.
///
/// `delivered` is the last record this session sent the peer, or `None`
/// when it has sent none; `next` is the first record to arrive after the
/// gap. Pure, and the whole of the rule. The count a lagged subscription
/// reports says how many messages were skipped and neither endpoint, which
/// is why both ends are derived from records this session can name - and
/// why the far end is only knowable here, at the record that ends the gap.
///
/// `None` when there is no gap to state: nothing was skipped before
/// `next`, which happens when `next` is the record `delivered` already
/// named or when it is the first record there is. A sequence number that
/// cannot be stepped returns `None` for the same reason and is a case a
/// ledger which produced `next` cannot reach.
#[must_use]
pub(crate) fn decide_lag(delivered: Option<Seq>, next: Seq) -> Option<Lagged> {
    let from = match delivered {
        Some(last) => last.value().checked_add(1).map(Seq::new),
        // A session that has delivered nothing has shown the peer no
        // stream, so the gap relative to it starts at the first record
        // the city ever wrote.
        None => Some(Seq::FIRST),
    };
    let to = next.value().checked_sub(1).map(Seq::new);
    match (from, to) {
        (Some(from), Some(to)) if to >= from => Some(Lagged { from, to }),
        _ => None,
    }
}

/// What a session owes the peer about the event stream.
///
/// Two facts that belong together: the last record this session delivered,
/// and whether the stream skipped past it since. They travel as one value
/// because every other combination is meaningless - a session that owes a
/// range has a delivery to name the near end of it, and one that owes none
/// has said everything it holds. The rule lives here rather than in the
/// shell for the reason every other judgement does: it is testable without
/// a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    /// The peer holds every record this session sent it.
    Even(Option<Seq>),
    /// The stream skipped records, and the next one to arrive names the far
    /// end of what was skipped. The payload is the last delivery.
    Owed(Option<Seq>),
}

impl Stream {
    /// A session that has delivered nothing and owes nothing, which is how
    /// every session starts.
    pub(crate) const fn opening() -> Stream {
        Stream::Even(None)
    }

    /// The record this session last delivered, from either state.
    #[must_use]
    pub(crate) fn delivered(self) -> Option<Seq> {
        match self {
            Stream::Even(delivered) | Stream::Owed(delivered) => delivered,
        }
    }

    /// The state a skip leaves behind. A session that has not been
    /// welcomed has shown the peer no stream to measure a gap against - its
    /// view begins at the welcome, and what a page needs of what came
    /// before is a question rather than a frame - so it owes nothing.
    #[must_use]
    pub(crate) fn skipped(self, live: bool) -> Stream {
        if live {
            Stream::Owed(self.delivered())
        } else {
            self
        }
    }

    /// What to say before sending `next`, and the state that sending it
    /// leaves behind. The far end of a skipped range is only knowable at
    /// the record that ends it.
    #[must_use]
    pub(crate) fn before(self, next: Seq) -> (Option<Lagged>, Stream) {
        let lag = match self {
            Stream::Even(_) => None,
            Stream::Owed(delivered) => decide_lag(delivered, next),
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
