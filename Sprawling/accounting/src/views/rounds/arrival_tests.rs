// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

//! A word that arrived and a reply wait, as the rounds answer them
//! (wire D36-D38, accounting D48): who spoke, what, when, and how a wait
//! ended.

use super::turns;
use wire::{B3Hash, EventDraft, EventKind, EventRecord, Note, Payload, RunId, Seq, TimeMs};

fn record(seq: u64, kind: EventKind, data: serde_json::Value) -> EventRecord {
    let map = data.as_object().expect("a payload is an object").clone();
    EventRecord::from_draft(
        EventDraft {
            run: RunId::from_bytes([7u8; 16]),
            t: TimeMs::new(seq),
            who: "lab/parser".to_owned(),
            addr: None,
            kind,
            data: Payload::new(map).expect("a payload"),
            ig: false,
        },
        Seq::new(seq),
        B3Hash::digest(b"prev"),
    )
}

fn asked(seq: u64) -> EventRecord {
    record(seq, EventKind::ModelCalled, serde_json::json!({}))
}

#[test]
fn a_word_from_a_person_lands_in_the_turn_it_reached() {
    let events = [
        asked(1),
        record(
            2,
            EventKind::SteerReceived,
            serde_json::json!({ "source": "user", "text": "ignore the cache" }),
        ),
    ];
    assert_eq!(
        turns(&events)[0].notes,
        vec![Note::Arrived {
            from: Some("user".to_owned()),
            said: Some("ignore the cache".to_owned()),
            by: wire::Speaker::User,
            t: TimeMs::new(2),
            handback: None,
            kind: None,
            session: None,
            at: Seq::new(2),
        }],
        "the User's steer says the User spoke, what, and when"
    );
}

#[test]
fn a_steer_of_another_shape_is_unreadable_rather_than_silent() {
    let events = [
        asked(1),
        record(
            2,
            EventKind::SteerReceived,
            serde_json::json!({ "source": "user", "said": "ignore the cache" }),
        ),
    ];
    match turns(&events)[0].notes.as_slice() {
        [Note::Unreadable { cause, at }] => {
            assert!(cause.starts_with("SteerReceived"), "{cause}");
            assert_eq!(*at, Seq::new(2));
        }
        other => panic!("somebody spoke, and the failure to read it stays, got {other:?}"),
    }
}

/// One line drafted under `run` by `who`.
fn drafted(run: RunId, who: &str, kind: EventKind, data: Payload) -> EventDraft {
    EventDraft {
        run,
        t: TimeMs::new(9),
        who: who.to_owned(),
        addr: None,
        kind,
        data,
        ig: false,
    }
}

/// The notes on the receiving session's first turn, after `signal` was
/// sent under `sender`'s own run and pulled in the receiving session,
/// as the production door answers them.
fn pulled_in_another_session(
    signal: &collab::Signal,
    sender: RunId,
    from: &str,
) -> Vec<wire::Note> {
    use kernel::Ledger;
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let receiver = RunId::from_bytes([2u8; 16]);
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(9))
        .unwrap()
        .0;
    for draft in [
        drafted(
            sender,
            from,
            EventKind::SignalEnqueued,
            signal.enqueued_payload().unwrap(),
        ),
        drafted(receiver, "lab/b", EventKind::ModelCalled, Payload::empty()),
        drafted(
            receiver,
            "lab/b",
            EventKind::SignalConsumed,
            signal.consumed_payload("lab/b").unwrap(),
        ),
    ] {
        ledger.append(draft).unwrap();
    }
    drop(ledger);
    let mut views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();
    let wire::Answer::Rounds(answer) = views.answer(&wire::Query::Rounds { run: receiver }) else {
        panic!("Rounds answers with rounds");
    };
    answer.turns[0].notes.clone()
}

/// A sends to B's room under A's own run; B pulls it in its session.
/// The rounds answer for B says A spoke, and what A said (wire D36).
#[test]
fn a_signal_pulled_from_another_session_says_who_sent_it_and_what() {
    use kernel::event::record::{SignalId, SignalKind};
    let mut said = serde_json::Map::new();
    said.insert(
        "text".to_owned(),
        serde_json::json!("the lexer is yours now"),
    );
    let signal = collab::Signal::new(
        SignalId::parse("a-s1").unwrap(),
        SignalKind::Mention,
        "lab/a".to_owned(),
        kernel::Address::parse("lab/b").unwrap(),
        kernel::Version::FIRST,
        Payload::new(said).unwrap(),
        TimeMs::new(9),
    )
    .unwrap();
    let notes = pulled_in_another_session(&signal, RunId::from_bytes([1u8; 16]), "lab/a");
    assert_eq!(
        notes,
        vec![wire::Note::Arrived {
            from: Some("lab/a".to_owned()),
            said: Some("the lexer is yours now".to_owned()),
            by: wire::Speaker::Resident,
            t: TimeMs::new(9),
            handback: None,
            kind: Some(SignalKind::Mention),
            session: Some(RunId::from_bytes([1u8; 16])),
            at: notes[0].at(),
        }],
        "the sender, its words, the kind it sent and the session it sent from (wire D43)"
    );
}

/// A child session hands its work back; the parent's arrival says it
/// finished, who verified it, and which session it was (wire D38, D43).
#[test]
fn a_handback_says_whether_the_child_finished() {
    let child = RunId::from_bytes([3u8; 16]);
    let claim = collab::Claim::new(
        collab::NodeId::parse("lab/b/lexer").unwrap(),
        kernel::Locator::cas(B3Hash::digest(b"account")),
        B3Hash::digest(b"account"),
        "lab/child".to_owned(),
    );
    let signal = collab::Handback::of(claim, true, "city")
        .signal(
            kernel::event::record::SignalId::parse("handback-child").unwrap(),
            kernel::Address::parse("lab/b").unwrap(),
            TimeMs::new(9),
        )
        .unwrap();
    let notes = pulled_in_another_session(&signal, child, "lab/child");
    match notes.as_slice() {
        [
            wire::Note::Arrived {
                from,
                handback,
                session,
                ..
            },
        ] => {
            assert_eq!(from.as_deref(), Some("lab/child"));
            assert_eq!(
                *handback,
                Some(wire::HandbackNote::Finished {
                    verified_by: "city".to_owned(),
                })
            );
            assert_eq!(*session, Some(child), "the child's session (wire D43)");
        }
        other => panic!("one arrival on the turn, got {other:?}"),
    }
}

/// A turn that sent with `wait` says whom it waits on, until when, and
/// how the wait ended (wire D37).
#[test]
fn a_reply_wait_says_how_it_ended() {
    let events = [
        asked(1),
        record(
            2,
            EventKind::SignalWaitStarted,
            serde_json::json!({ "on": "lab/a", "signal": "b-s1", "deadline_ms": 60_000 }),
        ),
        record(
            3,
            EventKind::SignalWaitEnded,
            serde_json::json!({ "signal": "b-s1", "by": { "end": "timeout" } }),
        ),
    ];
    let mut folded = turns(&events);
    super::paired::end_reply_waits(&mut folded, &events);
    assert_eq!(
        folded[0].notes,
        vec![wire::Note::AwaitingReply {
            on: kernel::Address::parse("lab/a").unwrap(),
            until: TimeMs::new(60_000),
            t: TimeMs::new(2),
            ended: Some(wire::ReplyEnded {
                by: wire::ReplyEnd::Timeout,
                t: TimeMs::new(3),
            }),
            at: Seq::new(2),
        }]
    );
}
