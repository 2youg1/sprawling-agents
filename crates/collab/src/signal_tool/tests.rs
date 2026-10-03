// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// What the post was handed, in order.
type Posted = Arc<Mutex<Vec<SignalEffect>>>;

fn desk(room: &str, reach: &str) -> Arc<Mutex<SignalDesk>> {
    desk_posting(room, reach).0
}

fn desk_posting(room: &str, reach: &str) -> (Arc<Mutex<SignalDesk>>, Posted, Mailslot) {
    let posted = Posted::default();
    let slot = Mailslot::default();
    let into = Arc::clone(&posted);
    let desk = SignalDesk::new(
        RunId::CITY,
        Address::parse(room).unwrap(),
        "potter@lab.1".to_owned(),
        Address::parse(reach).unwrap(),
        TimeMs::new(1_700_000_000_000),
        RoomMail {
            inbox: Inbox::new(64, 4),
            slot: slot.clone(),
            post: Post::new(move |line| {
                into.lock().unwrap().push(line.clone());
                Ok(())
            }),
        },
    );
    (Arc::new(Mutex::new(desk)), posted, slot)
}

fn call(args: Value) -> ToolCall {
    ToolCall {
        id: "tu_1".to_owned(),
        name: ToolName::parse("signal").unwrap(),
        args: Payload::new(args.as_object().unwrap().clone()).unwrap(),
    }
}

/// D7: a send is posted at the call, not left on the desk for later.
#[test]
fn sending_posts_the_signal_at_the_call() {
    let (shared, posted, _) = desk_posting("lab/room1", "lab");
    let tool = SignalTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({
        "action": "send",
        "to": "lab/room2",
        "text": "the kiln is free",
    })))
    .unwrap();
    assert_eq!(
        shared.lock().unwrap().pending(),
        0,
        "the sender's own inbox is not where a sent signal goes"
    );
    let effects = posted.lock().unwrap().clone();
    assert_eq!(effects.len(), 1);
    match &effects[0] {
        SignalEffect::Enqueued(signal) => {
            assert_eq!(signal.room().as_str(), "lab/room2");
            assert_eq!(signal.from(), "potter@lab.1");
        }
        other => panic!("a send posts an enqueue, not {other:?}"),
    }
}

#[test]
fn a_signal_addressed_outside_the_building_is_refused_with_somewhere_to_go() {
    let (shared, posted, _) = desk_posting("lab/room1", "lab");
    let tool = SignalTool::new(Arc::clone(&shared)).unwrap();
    let refusal = tool
        .invoke(&call(serde_json::json!({
            "action": "send",
            "to": "mill/room1",
            "text": "hello",
        })))
        .unwrap_err();
    assert_eq!(refusal.code(), &AxCode::CrossBuildingDenied);
    assert!(
        refusal.recovery().contains("lab"),
        "the third part of a refusal names where the caller may go instead"
    );
    assert!(
        posted.lock().unwrap().is_empty(),
        "a refused send leaves nothing behind"
    );
}

#[test]
fn pulling_takes_what_is_waiting_and_says_what_is_left() {
    let (shared, posted, _) = desk_posting("lab/room1", "lab");
    for n in 0..5u32 {
        let mut body = Map::new();
        body.insert("text".to_owned(), Value::String(format!("note {n}")));
        let signal = Signal::new(
            SignalId::parse(&format!("s{n}")).unwrap(),
            SignalKind::Mention,
            "mason@lab.2".to_owned(),
            Address::parse("lab/room1").unwrap(),
            Version::FIRST,
            Payload::new(body).unwrap(),
            TimeMs::new(10),
        )
        .unwrap();
        shared.lock().unwrap().inbox.deliver(&signal).unwrap();
    }
    let tool = SignalTool::new(Arc::clone(&shared)).unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({ "action": "pull" })))
        .unwrap();
    let map = outcome.result.as_map();
    assert_eq!(
        map.get("signals").and_then(Value::as_array).unwrap().len(),
        4,
        "a pull takes the receiver's bandwidth, not the sender's volume"
    );
    assert_eq!(
        map.get("remaining").and_then(Value::as_u64),
        Some(1),
        "what is left is in the answer, because status only knows the dispatch"
    );
    assert!(
        posted.lock().unwrap().is_empty(),
        "pulled is held, not consumed, until an answer reads it (D8)"
    );
    shared.lock().unwrap().answered().unwrap();
    assert_eq!(posted.lock().unwrap().len(), 4);
}

#[test]
fn an_action_this_tool_does_not_have_is_refused_by_name() {
    let shared = desk("lab/room1", "lab");
    let tool = SignalTool::new(Arc::clone(&shared)).unwrap();
    let refusal = tool
        .invoke(&call(serde_json::json!({ "action": "broadcast_all" })))
        .unwrap_err();
    assert_eq!(refusal.code(), &AxCode::InvalidArgs);
    assert!(refusal.recovery().contains("pull"));
}

#[test]
fn the_lent_inbox_comes_back() {
    let shared = desk("lab/room1", "lab");
    let mut body = Map::new();
    body.insert("text".to_owned(), Value::String("later".to_owned()));
    let signal = Signal::new(
        SignalId::parse("s1").unwrap(),
        SignalKind::Mention,
        "mason@lab.2".to_owned(),
        Address::parse("lab/room1").unwrap(),
        Version::FIRST,
        Payload::new(body).unwrap(),
        TimeMs::new(10),
    )
    .unwrap();
    shared.lock().unwrap().inbox.deliver(&signal).unwrap();
    let returned = shared.lock().unwrap().take_inbox();
    assert_eq!(returned.pending(), 1);
    assert_eq!(
        shared.lock().unwrap().pending(),
        0,
        "what was handed back is no longer held twice"
    );
}

/// A signal the run cannot read is not the same fact as an empty queue.
///
/// A steer-kind signal whose payload carries no words goes into the urgent
/// line and then fails `Steer::from_signal`. Answering "nothing waiting"
/// with that signal already out of the queue and no effect written would
/// make the history say the steer never arrived: the sender would see a
/// queued signal and the receiver silence. One assertion per
/// half of that - the refusal is reported, and the consumption is
/// recorded.
#[test]
fn an_unreadable_steer_is_refused_and_still_recorded_as_consumed() {
    let (shared, posted, _) = desk_posting("lab/room1", "lab");
    let mut body = Map::new();
    body.insert("text".to_owned(), Value::String(String::new()));
    let signal = Signal::new(
        SignalId::parse("s1").unwrap(),
        SignalKind::Steer,
        "mason@lab.2".to_owned(),
        Address::parse("lab/room1").unwrap(),
        Version::FIRST,
        Payload::new(body).unwrap(),
        TimeMs::new(10),
    )
    .unwrap();
    shared.lock().unwrap().inbox.deliver(&signal).unwrap();

    let mut borrowed = shared.lock().unwrap();
    assert_eq!(borrowed.pending(), 1, "the steer is waiting to be taken");
    let taken = borrowed.take_steer();
    assert!(
        taken.is_err(),
        "a steer with no words is not one the run can read"
    );
    assert_eq!(borrowed.pending(), 0, "taking it left the queue");
    let effects = posted.lock().unwrap().clone();
    assert_eq!(
        effects.len(),
        1,
        "the consumption is what the ledger records"
    );
    match &effects[0] {
        SignalEffect::Consumed { signal, by } => {
            assert_eq!(signal.id().as_str(), "s1");
            assert_eq!(by, "potter@lab.1");
        }
        other => panic!("taking a signal consumes it, not {other:?}"),
    }
}

fn steer(id: &str, words: &str) -> Signal {
    let mut body = Map::new();
    body.insert("text".to_owned(), Value::String(words.to_owned()));
    Signal::new(
        SignalId::parse(id).unwrap(),
        SignalKind::Steer,
        "mason@lab.2".to_owned(),
        Address::parse("lab/room1").unwrap(),
        Version::FIRST,
        Payload::new(body).unwrap(),
        TimeMs::new(10),
    )
    .unwrap()
}

/// F8: a run that took a steer and left before any model answer read it
/// gives it back. The trace is `withoutRequeue_loses` in
/// `spec/Delivery.lean`: send, start, take, cancel.
#[test]
fn a_steer_taken_and_never_answered_goes_back_when_the_run_leaves() {
    let shared = desk("lab/room1", "lab");
    shared
        .lock()
        .unwrap()
        .inbox
        .deliver(&steer("s1", "stop and read the brief"))
        .unwrap();
    let mut borrowed = shared.lock().unwrap();
    assert!(borrowed.take_steer().unwrap().is_some());
    let returned = borrowed.take_inbox();
    assert_eq!(
        returned.pending(),
        1,
        "a signal no recorded answer read is still waiting for the room"
    );
}

/// D8: a steer read by a recorded answer is consumed once and does not
/// come back; one taken after that answer is still held and does.
#[test]
fn an_answered_steer_is_consumed_and_a_later_one_is_held() {
    let (shared, posted, slot) = desk_posting("lab/room1", "lab");
    slot.drop_in(steer("s1", "first"), 64).unwrap();
    let mut borrowed = shared.lock().unwrap();
    assert!(
        borrowed.take_steer().unwrap().is_some(),
        "what the city dropped in the slot arrives at the next safe point"
    );
    borrowed.answered().unwrap();
    slot.drop_in(steer("s2", "second"), 64).unwrap();
    assert!(borrowed.take_steer().unwrap().is_some());
    let consumed: Vec<String> = posted
        .lock()
        .unwrap()
        .iter()
        .map(|line| match line {
            SignalEffect::Consumed { signal, .. } => signal.id().as_str().to_owned(),
            SignalEffect::Enqueued(signal) => panic!("nothing was sent: {signal:?}"),
        })
        .collect();
    assert_eq!(consumed, vec!["s1".to_owned()]);
    let mut returned = borrowed.take_inbox();
    assert_eq!(
        borrowed.take_unread().len(),
        1,
        "the room is knocked for what went back"
    );
    assert_eq!(
        returned
            .take_steer()
            .map(|signal| signal.id().as_str().to_owned()),
        Some("s2".to_owned()),
        "only the unread steer went back"
    );
}

/// One step of `Collab.Delivery.Act` for one room, as the city drives
/// it: a send drops into the queue at home or the reader's slot, a
/// start lends the queue, a take is a safe point, an ack is a model
/// answer landing, a leave gives the queue back.
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

/// Where the room's queue is: at home, or lent to a reader's desk.
#[allow(clippy::large_enum_variant, reason = "one value per trace")]
enum Room {
    Home(Inbox),
    Lent { desk: SignalDesk, slot: Mailslot },
}

/// The ids each `signal_consumed` line names, in ledger order.
fn consumed(posted: &Posted) -> Vec<String> {
    posted
        .lock()
        .unwrap()
        .iter()
        .filter_map(|line| match line {
            SignalEffect::Consumed { signal, .. } => Some(signal.id().as_str().to_owned()),
            SignalEffect::Enqueued(_) => None,
        })
        .collect()
}

/// Steps one trace and checks the `Collab.Delivery.Inv` fields one room
/// can show after every step: `consumed_once` (no id consumed twice),
/// D8 (consumed is exactly what a take handed out before an answer
/// landed), and, whenever the queue is home, `no_loss` (every sent id
/// is consumed or still queued). Nothing reads a clock or the platform,
/// so a trace gives one result on Windows, macOS and Linux (collab D7).
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
    /// `spec/Delivery.lean` `no_loss`, `consumed_once`, `leave_requeues`
    /// over random traces of one room.
    #[test]
    fn a_room_loses_no_signal_and_consumes_none_twice(
        acts in proptest::collection::vec(act(), 0..48)
    ) {
        run_trace(&acts);
    }
}

/// The trace vector of F8: send, start, take, cancel. The signal is
/// back in the queue, unconsumed, and the leaving run reports it for
/// the one knock (`withoutRequeue_loses` is what this refuses).
#[test]
fn send_start_take_cancel_puts_the_signal_back() {
    run_trace(&[Act::Send, Act::Start, Act::Take, Act::Leave]);
}
