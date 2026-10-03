// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::{Tool, ToolCall, ToolName};

use super::*;
use crate::signal_tool::SignalTool;

/// What the post was handed, in order.
pub(super) type Posted = Arc<Mutex<Vec<SignalEffect>>>;

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
        other @ (SignalEffect::Consumed { .. }
        | SignalEffect::WaitStarted(_)
        | SignalEffect::WaitEnded(_)) => panic!("a send posts an enqueue, not {other:?}"),
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
        other @ (SignalEffect::Enqueued(_)
        | SignalEffect::WaitStarted(_)
        | SignalEffect::WaitEnded(_)) => panic!("taking a signal consumes it, not {other:?}"),
    }
}

pub(super) fn steer(id: &str, words: &str) -> Signal {
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
            SignalEffect::WaitStarted(_) | SignalEffect::WaitEnded(_) => {
                panic!("nothing waited: {line:?}")
            }
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

/// The tool's words sit in the cached prefix of every request and are
/// read by a model as written: a line broken inside a string literal
/// without its continuation once put a run of spaces into `wait`.
#[test]
fn the_tool_speaks_to_the_model_without_runs_of_spaces() {
    fn words(value: &Value, into: &mut Vec<String>) {
        match value {
            Value::String(text) => into.push(text.clone()),
            Value::Array(items) => items.iter().for_each(|item| words(item, into)),
            Value::Object(map) => map.values().for_each(|item| words(item, into)),
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
    let tool = SignalTool::new(desk("lab/room1", "lab")).unwrap();
    let mut said = vec![tool.meta().disclosure.clone()];
    words(
        &Value::Object(tool.meta().params.as_map().clone()),
        &mut said,
    );
    let spaced: Vec<&String> = said.iter().filter(|text| text.contains("  ")).collect();
    assert!(spaced.is_empty(), "{spaced:?}");
}
