// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn signal(id: &str, kind: SignalKind) -> Signal {
    let mut body = Map::new();
    body.insert("text".to_owned(), Value::String(format!("body of {id}")));
    Signal::new(
        SignalId::parse(id).unwrap(),
        kind,
        "lab/room1".to_owned(),
        Address::parse("lab/room2").unwrap(),
        Version::new(3),
        Payload::new(body).unwrap(),
        TimeMs::new(1_000),
    )
    .unwrap()
}

#[test]
fn the_same_signal_delivered_twice_is_taken_once() {
    let mut inbox = Inbox::new(16, 8);
    assert!(matches!(
        inbox.deliver(&signal("s-1", SignalKind::Mention)).unwrap(),
        Admission::Admit
    ));
    assert!(
        matches!(
            inbox.deliver(&signal("s-1", SignalKind::Mention)).unwrap(),
            Admission::Admit
        ),
        "a second delivery is the sender doing its job, not an error"
    );
    assert_eq!(inbox.pending(), 1);
    assert_eq!(inbox.pull().unwrap().len(), 1);

    // And a copy arriving after consumption is still recognised: the
    // side effect already ran once.
    inbox.deliver(&signal("s-1", SignalKind::Mention)).unwrap();
    assert!(inbox.pull().unwrap().is_empty());
}

#[test]
fn a_steer_overtakes_what_was_already_waiting() {
    let mut inbox = Inbox::new(16, 8);
    inbox.deliver(&signal("s-1", SignalKind::Mention)).unwrap();
    inbox.deliver(&signal("s-2", SignalKind::Thread)).unwrap();
    inbox.deliver(&signal("s-3", SignalKind::Steer)).unwrap();

    let taken = inbox.pull().unwrap();
    let order: Vec<&str> = taken.iter().map(|s| s.id().as_str()).collect();
    assert_eq!(order, ["s-3", "s-1", "s-2"]);
}

#[test]
fn a_receiver_takes_its_own_bandwidth_and_no_more() {
    let mut inbox = Inbox::new(16, 2);
    for n in 1..=5 {
        inbox
            .deliver(&signal(&format!("s-{n}"), SignalKind::Mention))
            .unwrap();
    }
    assert_eq!(inbox.pending(), 5);
    assert_eq!(inbox.pull().unwrap().len(), 2);
    assert_eq!(inbox.pending(), 3, "the rest wait; nobody pushed them out");
    assert_eq!(inbox.pull().unwrap().len(), 2);
    assert_eq!(inbox.pull().unwrap().len(), 1);
    assert!(inbox.pull().unwrap().is_empty());
}

#[test]
fn a_flood_is_shed_at_the_door_and_what_is_queued_still_gets_out() {
    let mut inbox = Inbox::new(4, 8);
    let mut shed = 0;
    for n in 1..=64 {
        match inbox
            .deliver(&signal(&format!("s-{n}"), SignalKind::Mention))
            .unwrap()
        {
            Admission::Admit => {}
            Admission::Shed { .. } => shed += 1,
        }
    }
    assert!(shed > 0, "a flood is refused at the door");
    let taken = inbox.pull().unwrap();
    assert!(
        !taken.is_empty(),
        "shedding refuses new items and never starves the queued ones"
    );
}

#[test]
fn a_signal_survives_the_queue_byte_for_byte() {
    let mut inbox = Inbox::new(16, 8);
    let sent = signal("s-1", SignalKind::Thread);
    inbox.deliver(&sent).unwrap();
    let taken = inbox.pull().unwrap();
    assert_eq!(taken.len(), 1);
    assert_eq!(taken[0], sent);
}

#[test]
fn the_records_say_which_line_it_took_and_who_took_it() {
    let sent = signal("s-1", SignalKind::Steer);
    let enqueued = sent.enqueued_payload().unwrap();
    assert_eq!(
        enqueued.as_map().get("lane").and_then(Value::as_str),
        Some("urgent")
    );
    assert_eq!(
        enqueued.as_map().get("kind").and_then(Value::as_str),
        Some("steer")
    );

    let consumed = sent.consumed_payload("lab/room2").unwrap();
    assert_eq!(consumed.as_map().len(), 2);
    assert_eq!(
        consumed.as_map().get("by").and_then(Value::as_str),
        Some("lab/room2")
    );
}

#[test]
fn a_signal_from_nobody_is_not_a_signal() {
    let err = Signal::new(
        SignalId::parse("s-1").unwrap(),
        SignalKind::Mention,
        String::new(),
        Address::parse("lab").unwrap(),
        Version::new(1),
        Payload::empty(),
        TimeMs::new(1),
    )
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(SignalId::parse("s 1").is_err());
    assert!(SignalId::parse("").is_err());
}
