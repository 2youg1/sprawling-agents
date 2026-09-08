// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn desk(room: &str, reach: &str) -> Arc<Mutex<SignalDesk>> {
    Arc::new(Mutex::new(SignalDesk::new(
        RunId::CITY,
        Address::parse(room).unwrap(),
        "potter@lab.1".to_owned(),
        Address::parse(reach).unwrap(),
        TimeMs::new(1_700_000_000_000),
        Inbox::new(64, 4),
    )))
}

fn call(args: Value) -> ToolCall {
    ToolCall {
        id: "tu_1".to_owned(),
        name: ToolName::parse("signal").unwrap(),
        args: Payload::new(args.as_object().unwrap().clone()).unwrap(),
    }
}

#[test]
fn sending_queues_an_effect_and_delivers_nothing_yet() {
    let shared = desk("lab/room1", "lab");
    let mut tool = SignalTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({
        "action": "send",
        "to": "lab/room2",
        "text": "the kiln is free",
    })))
    .unwrap();
    let mut borrowed = shared.lock().unwrap();
    assert_eq!(
        borrowed.pending(),
        0,
        "the sender's own inbox is not where a sent signal goes"
    );
    let effects = borrowed.take_effects();
    assert_eq!(effects.len(), 1);
    match &effects[0] {
        SignalEffect::Enqueued(signal) => {
            assert_eq!(signal.room().as_str(), "lab/room2");
            assert_eq!(signal.from(), "potter@lab.1");
        }
        other => panic!("a send queues an enqueue, not {other:?}"),
    }
    assert!(
        borrowed.take_effects().is_empty(),
        "an effect read twice would be a signal delivered twice"
    );
}

#[test]
fn a_signal_addressed_outside_the_building_is_refused_with_somewhere_to_go() {
    let shared = desk("lab/room1", "lab");
    let mut tool = SignalTool::new(Arc::clone(&shared)).unwrap();
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
        shared.lock().unwrap().take_effects().is_empty(),
        "a refused send leaves nothing behind"
    );
}

#[test]
fn pulling_takes_what_is_waiting_and_says_what_is_left() {
    let shared = desk("lab/room1", "lab");
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
    let mut tool = SignalTool::new(Arc::clone(&shared)).unwrap();
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
    assert_eq!(shared.lock().unwrap().take_effects().len(), 4);
}

#[test]
fn an_action_this_tool_does_not_have_is_refused_by_name() {
    let shared = desk("lab/room1", "lab");
    let mut tool = SignalTool::new(Arc::clone(&shared)).unwrap();
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
