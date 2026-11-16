// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use super::tests::{Posted, steer};
use super::*;

#[derive(Debug, Clone)]
enum Act {
    Send,
    Start,
    Take,
    Ack,
    Leave,
}

fn act() -> impl proptest::strategy::Strategy<Value = Act> {
    proptest::prop_oneof![
        proptest::strategy::Just(Act::Send),
        proptest::strategy::Just(Act::Start),
        proptest::strategy::Just(Act::Take),
        proptest::strategy::Just(Act::Ack),
        proptest::strategy::Just(Act::Leave),
    ]
}

#[allow(clippy::large_enum_variant, reason = "one value per trace")]
enum Room {
    Home(Inbox),
    Lent { desk: SignalDesk, slot: Mailslot },
}

fn consumed(posted: &Posted) -> Vec<String> {
    posted
        .lock()
        .unwrap()
        .iter()
        .filter_map(|line| match line {
            SignalEffect::Consumed { signal, .. } => Some(signal.id().as_str().to_owned()),
            SignalEffect::Enqueued(_)
            | SignalEffect::WaitStarted(_)
            | SignalEffect::WaitEnded(_) => None,
        })
        .collect()
}

fn run_trace(acts: &[Act]) {
    let posted = Posted::default();
    let mut room = Room::Home(Inbox::new(64, 4));
    let (mut sent, mut read, mut answered) = (0u32, Vec::<String>::new(), Vec::<String>::new());
    for act in acts {
        room = match (act, room) {
            (Act::Send, Room::Home(mut inbox)) => {
                sent += 1;
                inbox
                    .deliver(&steer(&format!("s{sent}"), &format!("s{sent}")))
                    .unwrap();
                Room::Home(inbox)
            }
            (Act::Send, Room::Lent { desk, slot }) => {
                sent += 1;
                slot.drop_in(steer(&format!("s{sent}"), &format!("s{sent}")), 64)
                    .unwrap();
                Room::Lent { desk, slot }
            }
            (Act::Start, Room::Home(inbox)) => {
                let slot = Mailslot::default();
                let into = Arc::clone(&posted);
                let desk = SignalDesk::new(
                    RunId::CITY,
                    Address::parse("lab/room1").unwrap(),
                    "potter@lab.1".to_owned(),
                    Address::parse("lab").unwrap(),
                    TimeMs::new(1),
                    RoomMail {
                        inbox,
                        slot: slot.clone(),
                        post: Post::new(move |line| {
                            into.lock().unwrap().push(line.clone());
                            Ok(())
                        }),
                    },
                );
                Room::Lent { desk, slot }
            }
            (Act::Take, Room::Lent { mut desk, slot }) => {
                if let Some(taken) = desk.take_steer().unwrap() {
                    read.push(taken.text().to_owned());
                }
                Room::Lent { desk, slot }
            }
            (Act::Ack, Room::Lent { mut desk, slot }) => {
                desk.answered().unwrap();
                answered.append(&mut read);
                Room::Lent { desk, slot }
            }
            (Act::Leave, Room::Lent { mut desk, .. }) => {
                read.clear();
                Room::Home(desk.take_inbox())
            }
            (Act::Start, room @ Room::Lent { .. })
            | (Act::Take | Act::Ack | Act::Leave, room @ Room::Home(_)) => room,
        };
        let lines = consumed(&posted);
        let mut unique = lines.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), lines.len(), "consumed_once: {lines:?}");
        assert_eq!(lines, answered, "D8: consumed is what an answer read");
        if let Room::Home(inbox) = &room {
            let consumed_count = u32::try_from(lines.len()).unwrap();
            assert_eq!(
                sent,
                consumed_count + inbox.pending(),
                "no_loss after {act:?}"
            );
        }
    }
}

proptest::proptest! {

    #[test]
    fn a_room_loses_no_signal_and_consumes_none_twice(
        acts in proptest::collection::vec(act(), 0..48)
    ) {
        run_trace(&acts);
    }
}

#[test]
fn send_start_take_cancel_puts_the_signal_back() {
    run_trace(&[Act::Send, Act::Start, Act::Take, Act::Leave]);
}
