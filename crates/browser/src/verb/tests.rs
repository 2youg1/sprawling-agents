// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the eight actions are read from, and which frames each one puts
//! on the wire.

use super::*;
use crate::session::SessionRequest;
use serde_json::json;

fn args(value: Value) -> Payload {
    Payload::new(
        value
            .as_object()
            .cloned()
            .unwrap_or_else(serde_json::Map::new),
    )
    .expect("test arguments carry no floats")
}

fn page() -> PageSnapshot {
    PageSnapshot::read(
        3,
        &json!([
            { "role": "button", "name": "Place order" },
            { "role": "textbox", "name": "Quantity" },
        ]),
    )
    .expect("the tree is the shape a snapshot reads")
}

fn context() -> ContextId {
    ContextId::parse("c1").expect("a literal id is well-formed")
}

#[test]
fn the_eight_names_are_the_eight_actions_and_a_ninth_is_refused() {
    let names = [
        json!({ "action": "open", "url": "https://example.test/" }),
        json!({ "action": "snapshot" }),
        json!({ "action": "act", "generation": 3, "ref": "e1", "kind": "click" }),
        json!({ "action": "screenshot" }),
        json!({ "action": "measure", "refs": ["e1"] }),
        json!({ "action": "console" }),
        json!({ "action": "viewport", "width": 1280, "height": 800 }),
        json!({ "action": "close" }),
    ];
    let mut read = Vec::new();
    for name in names {
        read.push(Verb::read(&args(name)).expect("each of the eight reads"));
    }
    assert_eq!(read.len(), 8);
    let err = Verb::read(&args(json!({ "action": "scroll" }))).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("snapshot"));
}

#[test]
fn opening_a_page_navigates_and_then_installs_the_recorder() {
    let mut session = Session::new();
    let verb = Verb::read(&args(
        json!({ "action": "open", "url": "https://example.test/" }),
    ))
    .unwrap();
    let frames = verb.frames(&mut session, &context(), None).unwrap();
    assert_eq!(frames.len(), 2, "a console nobody records cannot be read");
    assert_eq!(frames[0].method(), "browsingContext.navigate");
    assert_eq!(frames[1].method(), "script.evaluate");
    assert!(
        frames[1].to_wire().contains("window.__sprawling_console"),
        "the recorder a script installs is the one the console reads"
    );
}

#[test]
fn acting_on_a_page_nobody_looked_at_cannot_be_asked_for() {
    let mut session = Session::new();
    let verb = Verb::read(&args(
        json!({ "action": "act", "generation": 3, "ref": "e1", "kind": "click" }),
    ))
    .unwrap();
    let err = verb.frames(&mut session, &context(), None).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("snapshot"));
    let frames = verb
        .frames(&mut session, &context(), Some(&page()))
        .unwrap();
    assert_eq!(frames.len(), 1);
    assert!(frames[0].to_wire().contains("click()"));
}

#[test]
fn a_reference_the_model_invented_is_refused_before_it_reaches_the_page() {
    let mut session = Session::new();
    let verb = Verb::read(&args(json!({ "action": "measure", "refs": ["e1", "e9"] }))).unwrap();
    let err = verb
        .frames(&mut session, &context(), Some(&page()))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    let good = Verb::read(&args(json!({ "action": "measure", "refs": ["e2"] }))).unwrap();
    let frames = good
        .frames(&mut session, &context(), Some(&page()))
        .unwrap();
    let wire = frames[0].to_wire();
    assert!(wire.contains("getBoundingClientRect"), "{wire}");
    assert!(
        wire.contains("textbox"),
        "the second reference is the textbox, not the first button: {wire}"
    );
}

#[test]
fn a_viewport_and_a_close_are_one_frame_each_and_name_their_command() {
    let mut session = Session::new();
    let viewport = Verb::read(&args(
        json!({ "action": "viewport", "width": 1280, "height": 800 }),
    ))
    .unwrap();
    let frames = viewport.frames(&mut session, &context(), None).unwrap();
    assert_eq!(frames[0].method(), "browsingContext.setViewport");
    assert!(frames[0].to_wire().contains("1280"));
    let close = Verb::read(&args(json!({ "action": "close" }))).unwrap();
    let ended = close.frames(&mut session, &context(), None).unwrap();
    assert_eq!(ended[0].method(), "session.end");
}

#[test]
fn every_frame_of_one_conversation_carries_its_own_number() {
    let mut session = Session::new();
    let _ = session.begin(SessionRequest::default()).unwrap();
    let open = Verb::read(&args(
        json!({ "action": "open", "url": "https://example.test/" }),
    ))
    .unwrap()
    .frames(&mut session, &context(), None)
    .unwrap();
    let look = Verb::read(&args(json!({ "action": "snapshot" })))
        .unwrap()
        .frames(&mut session, &context(), None)
        .unwrap();
    let ids: Vec<u64> = open.iter().chain(look.iter()).map(Frame::id).collect();
    assert_eq!(ids, vec![2, 3, 4], "a renumbered frame loses its reply");
}

#[test]
fn a_script_answers_in_one_string_and_anything_else_is_refused() {
    let value =
        read_json(&json!({ "result": { "type": "string", "value": "[{\"role\":\"button\"}]" } }))
            .unwrap();
    assert_eq!(value, json!([{ "role": "button" }]));
    for wrong in [
        json!({}),
        json!({ "result": { "type": "number", "value": 3 } }),
        json!({ "result": { "type": "string", "value": "not json" } }),
    ] {
        assert_eq!(read_json(&wrong).unwrap_err().code(), &AxCode::WireMismatch);
    }
}

#[test]
fn only_an_error_entry_counts_as_the_page_complaining() {
    assert!(!complained(&json!([])));
    assert!(!complained(&json!([{ "level": "warn", "text": "slow" }])));
    assert!(complained(&json!([
        { "level": "log", "text": "hello" },
        { "level": "error", "text": "undefined is not a function" },
    ])));
    assert!(!complained(&json!("not an array")));
}
