// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Admission and reception: who is let in, and what a session's
//! first frames are allowed to be.

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
fn a_session_that_delivered_nothing_owes_the_whole_ledger_and_one_that_lost_nothing_owes_none() {
    let lag = decide_lag(None, Seq::new(9)).expect("nothing was delivered, so 0..=8 is missing");
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
