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

use super::*;

fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

fn spoken(id: &str, to: &Address) -> collab::Signal {
    collab::Signal::new(
        collab::SignalId::parse(id).unwrap(),
        collab::SignalKind::Mention,
        "lab/room2".to_owned(),
        to.clone(),
        kernel::Version::new(1),
        kernel::Payload::empty(),
        kernel::TimeMs::new(1),
    )
    .unwrap()
}

/// Two runs in one room used to take a queue each, and the one that
/// landed second wrote over the first: every signal the first had
/// collected disappeared although the ledger said it arrived. The
/// second run now works at a spare it returns nothing from.
#[test]
fn a_second_run_in_one_room_does_not_take_the_queue_away_from_the_first() {
    let mut rooms = RoomQueues::folded(std::collections::BTreeMap::new());
    let first = RunId::from_bytes([1u8; 16]);
    let second = RunId::from_bytes([2u8; 16]);
    let at = room();

    let held = rooms.lend(&at, first);
    assert!(matches!(held.tenure, QueueTenure::TheRoomQueue));
    let spare = rooms.lend(&at, second);
    match spare.tenure {
        QueueTenure::ASpare { held_by } => assert_eq!(held_by, first),
        QueueTenure::TheRoomQueue => panic!("one room lends one queue"),
    }

    // The run that never held it cannot give it back, however much it
    // has collected in its own spare.
    let refused = rooms.give_back(&at, second, spare.inbox).unwrap_err();
    assert_eq!(refused.code(), &AxCode::StorageFatal);
    rooms.give_back(&at, first, held.inbox).unwrap();
    assert!(rooms.at_home(&at).is_some(), "the holder gave it back");
}

/// What is said to a room while its queue is out used to land in a
/// fresh queue that the returning run then overwrote. It now waits in
/// the room's own entry and is delivered when the queue comes home.
#[test]
fn what_arrives_while_the_queue_is_out_is_waiting_when_it_comes_home() {
    let mut rooms = RoomQueues::folded(std::collections::BTreeMap::new());
    let driving = RunId::from_bytes([3u8; 16]);
    let at = room();

    let held = rooms.lend(&at, driving);
    rooms.deliver(&spoken("s-1", &at)).unwrap();
    rooms.deliver(&spoken("s-2", &at)).unwrap();
    assert_eq!(
        rooms.pending(&at),
        0,
        "a room whose queue is out reports what the reader holds, which is nothing here"
    );

    rooms.give_back(&at, driving, held.inbox).unwrap();
    assert_eq!(
        rooms.pending(&at),
        2,
        "both signals are in the queue the run gave back"
    );
}

/// A queue that failed on its way home used to be a queue the city
/// forgot it had. It comes home whatever the signals that waited do.
#[test]
fn the_queue_comes_home_even_when_a_signal_that_waited_is_refused() {
    let at = room();
    // One slot, so the second signal that waited cannot be admitted.
    let mut rooms = RoomQueues::folded(std::collections::BTreeMap::from([(
        at.clone(),
        collab::Inbox::new(1, 1),
    )]));
    let driving = RunId::from_bytes([3u8; 16]);
    let held = rooms.lend(&at, driving);
    rooms.deliver(&spoken("s-1", &at)).unwrap();
    rooms.deliver(&spoken("s-2", &at)).unwrap();

    let refused = rooms.give_back(&at, driving, held.inbox).unwrap_err();
    assert_eq!(refused.code(), &AxCode::BackpressureShed);
    assert_eq!(
        rooms.pending(&at),
        1,
        "the queue is home with what it could take"
    );
}
