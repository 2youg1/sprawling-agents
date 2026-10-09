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
//! built as (ARCHITECTURE section 9). `wire::server` is declared an
//! adapter — "thin, no policy" — and policy is what these functions
//! are; a shell that also holds the rules it applies has no way to show
//! that it applies all of them. Every branch left in the shell is a
//! send, a receive, or the end of a session.
//!
//! Binding is loopback by default, and the face that comes back from
//! [`decide_bind`] carries the key it demands: a listener refuses to
//! start without one, loopback included, so no shell can be built with a
//! face that demands nothing (wire D54).

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

/// Which face the listener presents, and the key it demands.
///
/// **The key travels inside both faces rather than beside them.** A
/// listener that demands nothing is not a state this type can hold,
/// loopback included: the loopback port is reachable by every page in a
/// browser on this machine, by every other account on it and by the
/// residents' own tools (`crates/wire/spec/Reception/Entry.lean` wire
/// D54). [`decide_bind`] is the only producer of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindFace {
    /// Reachable from this machine only.
    Loopback { key: B3Hash },
    /// Reachable from elsewhere.
    Exposed { key: B3Hash },
}

impl BindFace {
    /// The digest of the key this face demands of every caller that holds
    /// no session.
    #[must_use]
    pub fn key(&self) -> &B3Hash {
        match self {
            BindFace::Loopback { key } | BindFace::Exposed { key } => key,
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
/// Pure. Refuses exactly when no key is given, on either face, before
/// the socket exists - so there is no window in which the port is open
/// and unauthenticated - and the key the face demands comes back inside
/// the verdict, so there is no second way to learn it.
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
    // A `[::]` listener on Linux and macOS reports a local IPv4 caller as
    // `::ffff:127.0.0.1`, which is the same machine.
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
/// "refresh", not "wrong password". Pure - `expected` and `keys` are
/// parameters, never read from ambient state.
///
/// The credential a caller must present is the face's key or a live
/// session token ([`Keys::pairing`]); neither face can be served without
/// a key, so there is no greeting this accepts empty-handed.
///
/// The digest is a digest, not the token. This crate never holds the
/// plaintext of a pairing token: the side that owns the token digests it
/// once, and the boundary compares digests. That keeps credential exposure
/// at the redemption points where it is audited, and it costs nothing here
/// because the comparison hashes both sides anyway.
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
                "reload the page to sign in again; a program on this machine reads the key                  from the city's key file, or takes --token",
            ),
        ),
    }
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
    /// Evaluate this question and answer it under its own number.
    Answer(Box<Ask>),
    /// Count this session as watching the monitor and send it readings.
    Watch(Watched),
    /// Stop counting this session and stop sending it readings.
    Release,
    /// Sample the monitor at this beat, for every session of this city.
    Beat(crate::BeatMs),
    /// Send this refusal; `close` ends the session afterwards.
    Refuse { error: Box<AxError>, close: bool },
}

/// What a welcome tells a peer about the city it reached: which city it
/// is, the seq of the last record the city has broadcast, and which
/// ledger it is. One value
/// because the two travel together into every welcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WelcomeFacts<'a> {
    pub city: Option<&'a Address>,
    pub head: Option<Seq>,
    pub epoch: Option<B3Hash>,
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
            // A record that never reached the broadcast leaves only a gap
            // in `seq`; before anything was delivered there is no gap.
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
