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
mod tests {
    use super::*;

    /// The face a test session is judged against when no credential is
    /// configured: a city on this machine with nobody to distinguish.
    fn unpaired() -> BindFace {
        BindFace::Loopback { token: None }
    }

    #[test]
    fn an_ipv6_loopback_is_also_loopback() {
        let addr: SocketAddr = "[::1]:8787".parse().unwrap();
        assert!(matches!(
            decide_bind(&addr, None),
            BindVerdict::Serve(BindFace::Loopback { token: None })
        ));
    }

    #[test]
    fn only_a_caller_on_this_machine_may_enrol_a_credential() {
        for local in ["127.0.0.1:51000", "[::1]:51000"] {
            let peer: SocketAddr = local.parse().unwrap();
            assert!(matches!(decide_enroll(&peer), EnrollVerdict::Accept));
        }
        let remote: SocketAddr = "203.0.113.7:51000".parse().unwrap();
        let EnrollVerdict::Refuse(err) = decide_enroll(&remote) else {
            panic!("a peer beyond this machine cannot enrol a credential");
        };
        assert_eq!(*err.code(), AxCode::GateDenied);
        // The third part points at the one place it can be done, which is
        // what keeps this a constraint rather than a dead end.
        assert!(err.recovery().contains("machine running sprawling"));
    }

    #[test]
    fn a_pairing_token_does_not_buy_the_right_to_enrol() {
        // An exposed bind is legal with a token; enrolment still is not.
        let exposed: SocketAddr = "203.0.113.7:8787".parse().unwrap();
        let digest = B3Hash::digest(b"pairing-code");
        let BindVerdict::Serve(face) = decide_bind(&exposed, Some(digest)) else {
            panic!("an exposed bind with a token is served");
        };
        assert_eq!(face.token_digest(), Some(&digest));
        assert!(matches!(decide_enroll(&exposed), EnrollVerdict::Refuse(_)));
    }

    /// The face is the whole of what a session must present, and an
    /// exposed one carries the digest: there is no exposed face that
    /// demands nothing, so no shell can be built with one.
    #[test]
    fn a_city_reachable_from_elsewhere_never_serves_without_a_credential() {
        let exposed: SocketAddr = "203.0.113.7:8787".parse().unwrap();
        let BindVerdict::Refuse(err) = decide_bind(&exposed, None) else {
            panic!("an exposed bind with no token refuses to start");
        };
        assert_eq!(*err.code(), AxCode::ConfigInvalid);
        assert!(!err.recovery().is_empty());

        let digest = B3Hash::digest(b"pairing-code");
        let BindVerdict::Serve(opened) = decide_bind(&exposed, Some(digest)) else {
            panic!("an exposed bind with a token is served");
        };
        assert_eq!(
            opened.token_digest(),
            Some(&digest),
            "the digest the exposed face demands comes back inside the verdict"
        );

        // Loopback is served with or without a token, and a token a person
        // configured there is still demanded.
        let local: SocketAddr = "127.0.0.1:8787".parse().unwrap();
        let BindVerdict::Serve(paired) = decide_bind(&local, Some(digest)) else {
            panic!("a loopback bind is served");
        };
        assert_eq!(paired.token_digest(), Some(&digest));
        let BindVerdict::Serve(alone) = decide_bind(&local, None) else {
            panic!("a loopback bind needs no token");
        };
        assert_eq!(alone.token_digest(), None);
    }

    fn hello(wire_v: u32, token: Option<&str>) -> ClientFrame {
        ClientFrame::Hello(Hello {
            wire_v,
            schema: schema_hash(),
            token: token.map(str::to_owned),
        })
    }

    fn a_command() -> ClientFrame {
        ClientFrame::Command(Box::new(crate::command::Command::Cancel {
            run: kernel::RunId::CITY,
            idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"cancel"),
        }))
    }

    #[test]
    fn a_matching_hello_opens_the_session() {
        let step = decide_frame(
            SessionState::AwaitingHello,
            hello(WIRE_V, None),
            &unpaired(),
            None,
        );
        let SessionStep::Welcome(welcome) = step else {
            panic!("a matching hello is welcomed");
        };
        assert_eq!(welcome.wire_v, WIRE_V);
        assert_eq!(welcome.schema, schema_hash());
    }

    #[test]
    fn a_different_wire_closes_the_session_rather_than_negotiating() {
        let step = decide_frame(
            SessionState::AwaitingHello,
            hello(WIRE_V.saturating_add(1), None),
            &unpaired(),
            None,
        );
        let SessionStep::Refuse { error, close } = step else {
            panic!("a wire mismatch is refused");
        };
        assert!(close, "the session ends; two wire versions are two servers");
        assert!(!error.recovery().is_empty());
    }

    #[test]
    fn a_command_before_the_hello_is_refused_rather_than_queued() {
        let step = decide_frame(SessionState::AwaitingHello, a_command(), &unpaired(), None);
        let SessionStep::Refuse { close, .. } = step else {
            panic!("an unopened session runs nothing");
        };
        assert!(close);
    }

    #[test]
    fn a_live_session_delivers_commands_and_answers_queries() {
        assert!(matches!(
            decide_frame(SessionState::Live, a_command(), &unpaired(), None),
            SessionStep::Deliver(_)
        ));
        assert!(matches!(
            decide_frame(
                SessionState::Live,
                ClientFrame::Query(crate::wire::Query::CityView),
                &unpaired(),
                None
            ),
            SessionStep::Answer(_)
        ));
    }

    #[test]
    fn a_second_hello_is_refused_without_ending_the_session() {
        let step = decide_frame(SessionState::Live, hello(WIRE_V, None), &unpaired(), None);
        let SessionStep::Refuse { close, .. } = step else {
            panic!("one session, one greeting");
        };
        assert!(!close, "a confused client is corrected, not disconnected");
    }

    #[test]
    fn an_exposed_session_needs_the_pairing_token() {
        let exposed = BindFace::Exposed {
            token: B3Hash::digest(b"pairing-code"),
        };
        assert!(matches!(
            decide_frame(
                SessionState::AwaitingHello,
                hello(WIRE_V, None),
                &exposed,
                None
            ),
            SessionStep::Refuse { close: true, .. }
        ));
        assert!(matches!(
            decide_frame(
                SessionState::AwaitingHello,
                hello(WIRE_V, Some("pairing-code")),
                &exposed,
                None
            ),
            SessionStep::Welcome(_)
        ));
    }

    /// The far end of a skipped range is the record before the one that
    /// arrives after the gap: a subscription's count says how many
    /// messages went by and neither endpoint, so a session that reported
    /// the count alone would tell the peer nothing it could ask for.
    #[test]
    fn a_skipped_range_is_named_by_the_records_on_either_side_of_it() {
        let delivered = Seq::new(7);
        let lag = decide_lag(Some(delivered), Seq::new(20)).expect("records 8..=19 were skipped");
        assert_eq!(lag.from, Seq::new(8), "the record after the last delivered");
        assert_eq!(
            lag.to,
            Seq::new(19),
            "the record before the one that arrived"
        );
    }

    #[test]
    fn a_session_that_delivered_nothing_owes_the_whole_ledger_and_one_that_lost_nothing_owes_none()
    {
        let lag =
            decide_lag(None, Seq::new(9)).expect("nothing was delivered, so 0..=8 is missing");
        assert_eq!(lag.from, Seq::FIRST);
        assert_eq!(lag.to, Seq::new(8));
        // The record that arrives is the one already delivered, and the
        // first record there is has nothing before it: neither pair is a
        // gap, and saying so is not the same as staying silent.
        assert_eq!(decide_lag(Some(Seq::new(7)), Seq::new(8)), None);
        assert_eq!(decide_lag(None, Seq::FIRST), None);
        assert_eq!(decide_lag(Some(Seq::new(7)), Seq::new(7)), None);
    }

    /// The three moves a session makes around a skip: deliver, lose, report.
    ///
    /// This is the state machine the shell's select loop steps through, and
    /// it is here rather than there because a shell that also held the rule
    /// applied it would have no way to show that it applied all of it.
    #[test]
    fn a_skip_is_reported_once_and_the_debt_settles_at_the_next_record() {
        let (opening, delivered) = Stream::opening().before(Seq::new(7));
        assert_eq!(opening, None, "nothing is owed before anything is skipped");
        assert_eq!(delivered.delivered(), Some(Seq::new(7)));

        let owed = delivered.skipped(true);
        assert_eq!(
            owed.delivered(),
            Some(Seq::new(7)),
            "the near end of the range is what the peer holds"
        );
        let (lag, settled) = owed.before(Seq::new(20));
        assert_eq!(
            lag,
            Some(Lagged {
                from: Seq::new(8),
                to: Seq::new(19)
            })
        );
        assert_eq!(settled.delivered(), Some(Seq::new(20)));
        assert_eq!(
            settled.before(Seq::new(21)).0,
            None,
            "and the debt is paid once, not every record after it"
        );
    }

    #[test]
    fn a_session_that_has_not_been_welcomed_owes_no_range() {
        // Its view begins at the welcome; what a page needs of what came
        // before is a question it asks, not a frame it is owed.
        let skipped = Stream::opening().skipped(false);
        assert_eq!(skipped.delivered(), None);
        let (lag, _) = skipped.before(Seq::new(5));
        assert_eq!(lag, None);
    }

    #[test]
    fn an_unspecified_address_is_not_loopback() {
        // 0.0.0.0 reaches every interface; treating it as local would be the
        // exact mistake this judgement exists to prevent.
        let addr: SocketAddr = "0.0.0.0:8787".parse().unwrap();
        assert!(matches!(decide_bind(&addr, None), BindVerdict::Refuse(_)));
        assert!(matches!(
            decide_bind(&addr, Some(B3Hash::digest(b"pairing-code"))),
            BindVerdict::Serve(BindFace::Exposed { .. })
        ));
    }
}
