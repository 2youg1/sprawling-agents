// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! The ladder and the handshake, through the production door.

use channels::{Address, AxCode, Seq, ServerFrame, Welcome, schema_hash};

use super::frames::token_in;

use super::link::{Link, LinkAction, LinkEvent, LinkState, backoff_ms};

fn welcome_matching() -> Welcome {
    Welcome {
        wire_v: channels::WIRE_V,
        schema: schema_hash(),
        resume_from: Some(Seq::new(41)),
        city: Address::parse("kiln").ok(),
    }
}

/// The four answers a query string has. The city's own codes are
/// digits and lower-case letters in four hyphenated groups, so this
/// runs against the shape the host actually produces.
#[test]
fn the_pairing_code_is_read_off_the_url_the_host_opened() {
    assert_eq!(
        token_in("?token=hjkmn-pqrtu-vwxyz-23467"),
        Some("hjkmn-pqrtu-vwxyz-23467".to_owned())
    );
    assert_eq!(token_in("?view=city&token=abcde"), Some("abcde".to_owned()));
    assert_eq!(token_in("token=abcde"), Some("abcde".to_owned()));
    assert_eq!(token_in("?token="), None, "an empty value is not a value");
    assert_eq!(token_in(""), None);
    assert_eq!(token_in("?view=city"), None);
    assert_eq!(
        token_in("?tokenish=abcde"),
        None,
        "a name that merely starts like it is not it"
    );
}

/// **The defect this closes.** An exposed city asks every peer for a
/// pairing token, and the page had no way to have one: `app.rs`
/// built its link with `None`, so the one frame that could have
/// carried the code went out empty and the server refused its own
/// client.
#[test]
fn the_hello_carries_the_code_the_page_was_opened_with() {
    let mut link = Link::new(token_in("?token=hjkmn-pqrtu-vwxyz-23467"));
    assert_eq!(link.connect(), LinkAction::OpenSocket);
    let LinkAction::Send(hello) = link.advance(LinkEvent::Opened) else {
        panic!("an opened socket says hello");
    };
    assert_eq!(hello.token.as_deref(), Some("hjkmn-pqrtu-vwxyz-23467"));
}

#[test]
fn a_normal_join_opens_says_hello_and_goes_live() {
    let mut link = Link::new(None);
    assert_eq!(link.connect(), LinkAction::OpenSocket);
    assert!(matches!(
        link.advance(LinkEvent::Opened),
        LinkAction::Send(_)
    ));
    assert_eq!(
        link.advance(LinkEvent::Received(Box::new(ServerFrame::Welcome(
            welcome_matching()
        )))),
        LinkAction::Nothing
    );
    assert!(link.is_live());
    assert_eq!(
        link.state(),
        &LinkState::Live {
            resume_from: Some(Seq::new(41))
        }
    );
}

#[test]
fn a_stale_page_is_told_to_reload_and_stops_trying() {
    let mut link = Link::new(None);
    link.connect();
    link.advance(LinkEvent::Opened);
    let stale = Welcome {
        wire_v: channels::WIRE_V.saturating_add(1),
        schema: schema_hash(),
        resume_from: None,
        city: None,
    };
    let LinkAction::Report(err) =
        link.advance(LinkEvent::Received(Box::new(ServerFrame::Welcome(stale))))
    else {
        panic!("a version mismatch must be reported, not retried");
    };
    assert_eq!(*err.code(), AxCode::WireMismatch);
    assert!(err.recovery().contains("reload"));

    // Terminal: further transport noise must not restart the loop.
    assert_eq!(link.advance(LinkEvent::Closed), LinkAction::Nothing);
    assert_eq!(link.advance(LinkEvent::WaitElapsed), LinkAction::Nothing);
    assert!(!link.is_live());
}

#[test]
fn a_dropped_link_climbs_the_ladder_and_then_stays_flat() {
    let mut link = Link::new(None);
    link.connect();
    let mut waits = Vec::new();
    for _ in 0..8 {
        let LinkAction::WaitMs(ms) = link.advance(LinkEvent::Closed) else {
            panic!("a closed socket backs off");
        };
        waits.push(ms);
        link.advance(LinkEvent::WaitElapsed);
        link.advance(LinkEvent::TransportFailed);
        link.advance(LinkEvent::WaitElapsed);
    }
    assert_eq!(waits.first(), Some(&250));
    assert!(
        waits.windows(2).all(|pair| pair[1] >= pair[0]),
        "the ladder never goes back down: {waits:?}"
    );
    assert_eq!(waits.last(), Some(&10_000), "and it stops climbing");
}

#[test]
fn going_live_again_forgives_the_outage_that_preceded_it() {
    let mut link = Link::new(None);
    link.connect();
    for _ in 0..4 {
        link.advance(LinkEvent::Closed);
        link.advance(LinkEvent::WaitElapsed);
    }
    assert!(link.consecutive_failures() > 0);

    link.advance(LinkEvent::Opened);
    link.advance(LinkEvent::Received(Box::new(ServerFrame::Welcome(
        welcome_matching(),
    ))));
    assert!(link.is_live());
    assert_eq!(link.consecutive_failures(), 0);

    // The next outage starts at the bottom of the ladder.
    assert_eq!(link.advance(LinkEvent::Closed), LinkAction::WaitMs(250));
}

#[test]
fn the_ladder_is_a_total_function_of_the_attempt_number() {
    assert_eq!(backoff_ms(0), 250);
    assert_eq!(backoff_ms(5), 10_000);
    assert_eq!(
        backoff_ms(u32::MAX),
        10_000,
        "no index can escape the table"
    );
}

#[test]
fn events_advance_the_resume_cursor_so_a_reconnect_asks_for_less() {
    let mut link = Link::new(None);
    link.connect();
    link.advance(LinkEvent::Opened);
    link.advance(LinkEvent::Received(Box::new(ServerFrame::Welcome(
        welcome_matching(),
    ))));
    let event = channels::EventRecord::from_draft(
        channels::EventDraft {
            run: channels::RunId::from_bytes([1u8; 16]),
            t: channels::TimeMs::new(1),
            who: "server".to_owned(),
            addr: None,
            kind: channels::EventKind::RunStarted,
            data: channels::Payload::empty(),
            ig: false,
        },
        Seq::new(77),
        channels::B3Hash::digest(b"prev"),
    );
    assert!(matches!(
        link.advance(LinkEvent::Received(Box::new(ServerFrame::Event(Box::new(
            event
        ))))),
        LinkAction::Deliver(_)
    ));
    assert_eq!(
        link.state(),
        &LinkState::Live {
            resume_from: Some(Seq::new(77))
        }
    );
}

#[test]
fn a_server_that_streams_before_welcoming_is_not_this_protocol() {
    let mut link = Link::new(None);
    link.connect();
    link.advance(LinkEvent::Opened);
    let event = channels::EventRecord::from_draft(
        channels::EventDraft {
            run: channels::RunId::from_bytes([1u8; 16]),
            t: channels::TimeMs::new(1),
            who: "server".to_owned(),
            addr: None,
            kind: channels::EventKind::RunStarted,
            data: channels::Payload::empty(),
            ig: false,
        },
        Seq::new(1),
        channels::B3Hash::digest(b"prev"),
    );
    assert!(matches!(
        link.advance(LinkEvent::Received(Box::new(ServerFrame::Event(Box::new(
            event
        ))))),
        LinkAction::Report(_)
    ));
    assert!(matches!(link.state(), LinkState::Refused(_)));
}

#[test]
fn a_tab_nobody_is_looking_at_closes_rather_than_slowing_down() {
    let mut link = Link::new(None);
    assert_eq!(link.connect(), LinkAction::OpenSocket);
    link.advance(LinkEvent::Opened);
    assert_eq!(
        link.advance(LinkEvent::Backgrounded),
        LinkAction::CloseSocket,
        "a slowed tab still holds a socket and still wakes the machine"
    );
    assert!(matches!(link.state(), LinkState::Suspended { .. }));
}

#[test]
fn nothing_reaches_a_suspended_link_and_coming_back_starts_clean() {
    let mut link = Link::new(None);
    link.connect();
    link.advance(LinkEvent::Opened);
    link.advance(LinkEvent::Backgrounded);
    // The socket that was closing reports it; that must not restart
    // anything, and it must not count as a failure either.
    assert_eq!(link.advance(LinkEvent::Closed), LinkAction::Nothing);
    assert_eq!(link.advance(LinkEvent::WaitElapsed), LinkAction::Nothing);
    assert_eq!(link.consecutive_failures(), 0);
    assert_eq!(
        link.advance(LinkEvent::Foregrounded),
        LinkAction::OpenSocket,
        "there is a reader again"
    );
}

#[test]
fn a_refusal_does_not_become_less_true_because_somebody_switched_tabs() {
    let mut link = Link::new(None);
    link.connect();
    link.advance(LinkEvent::Opened);
    let stale = Welcome {
        wire_v: channels::WIRE_V.saturating_add(1),
        schema: schema_hash(),
        resume_from: None,
        city: None,
    };
    assert!(matches!(
        link.advance(LinkEvent::Received(Box::new(ServerFrame::Welcome(stale)))),
        LinkAction::Report(_)
    ));
    assert_eq!(link.advance(LinkEvent::Backgrounded), LinkAction::Nothing);
    assert!(matches!(link.state(), LinkState::Refused(_)));
}
