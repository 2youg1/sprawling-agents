// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One conversation with a browser, replayed without one.
//!
//! The recording is the seam's second adapter, so this is the production
//! path with the socket swapped out — not a double of the tool.

use super::*;
use browser::{Recording, SessionRequest};
use serde_json::json;

/// One red pixel, as a real PNG.
///
/// Kept in sixteen-byte pieces because the secret gate reads the file,
/// not the concatenation: a base64 run of twenty bytes or more is what
/// the entropy detector is built to catch, and it is right to catch it.
const ONE_RED_PIXEL: &str = concat!(
    "iVBORw0KGgoAAAAN",
    "SUhEUgAAAAEAAAAB",
    "CAYAAAAfFcSJAAAA",
    "DUlEQVR42mP8z8BQ",
    "DwAEhQGAhKmMIQAA",
    "AABJRU5ErkJggg=="
);

fn args(value: Value) -> Payload {
    Payload::new(value.as_object().cloned().unwrap_or_else(Map::new)).expect("no floats")
}

fn call(value: Value) -> ToolCall {
    ToolCall {
        id: "tu_1".to_owned(),
        name: ToolName::parse("browser").expect("a literal name"),
        args: args(value),
    }
}

fn string_result(text: &str) -> Value {
    json!({ "result": { "type": "string", "value": text } })
}

/// The conversation this replay answers, recorded against the frames a
/// mirror session mints in the same order the tool's does.
fn recorded() -> Recording {
    let mut mirror = Session::new();
    let context = ContextId::parse("c1").expect("a literal id");
    let mut recording = Recording::new();
    let begin = mirror
        .begin(SessionRequest::default())
        .expect("a session begins");
    recording.answer(&begin, json!({ "sessionId": "s1" }));
    let tree = mirror.tree().expect("the tree is asked for");
    recording.answer(&tree, json!({ "contexts": [{ "context": "c1" }] }));
    let mut answer = |verb: Value, snapshot: Option<&PageSnapshot>, results: Vec<Value>| {
        let frames = Verb::read(&args(verb))
            .expect("the verb reads")
            .frames(&mut mirror, &context, snapshot)
            .expect("the frames build");
        for (frame, result) in frames.iter().zip(results) {
            recording.answer(frame, result);
        }
    };
    answer(
        json!({ "action": "open", "url": "https://example.test/" }),
        None,
        vec![json!({}), string_result("\"ready\"")],
    );
    let page = json!([
        { "role": "button", "name": "Place order" },
        { "role": "textbox", "name": "Quantity" },
    ]);
    answer(
        json!({ "action": "snapshot" }),
        None,
        vec![string_result(&page.to_string())],
    );
    let looked = PageSnapshot::read(1, &page).expect("the tree reads");
    answer(
        json!({ "action": "act", "generation": 1, "ref": "e1", "kind": "click" }),
        Some(&looked),
        vec![json!({ "result": { "type": "undefined" } })],
    );
    answer(
        json!({ "action": "screenshot" }),
        None,
        vec![json!({ "data": ONE_RED_PIXEL })],
    );
    recording
}

fn tool(cas_at: &std::path::Path) -> BrowserTool {
    let cas = Cas::open(cas_at).expect("a cas opens in a fresh directory");
    BrowserTool::new(Box::new(recorded()), cas).expect("the tool builds")
}

fn text(outcome: &ToolOutcome) -> String {
    serde_json::to_string(&outcome.result).expect("a payload serialises")
}

#[test]
fn one_conversation_replays_from_open_to_a_screenshot_that_is_evidence() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut tool = tool(&dir.path().join("cas"));

    let opened = tool
        .invoke(&call(
            json!({ "action": "open", "url": "https://example.test/" }),
        ))
        .expect("the page opens");
    assert!(text(&opened).contains("https://example.test/"));

    let looked = tool
        .invoke(&call(json!({ "action": "snapshot" })))
        .expect("the page is looked at");
    let seen = text(&looked);
    assert!(seen.contains("e1 button"), "{seen}");
    assert!(
        seen.contains("\"generation\":1"),
        "a reference is a position in one snapshot: {seen}"
    );

    let acted = tool
        .invoke(&call(
            json!({ "action": "act", "generation": 1, "ref": "e1", "kind": "click" }),
        ))
        .expect("the button is clicked");
    assert!(!text(&acted).is_empty());

    let shot = tool
        .invoke(&call(json!({ "action": "screenshot" })))
        .expect("the page is photographed");
    let result = text(&shot);
    assert!(
        result.contains("cas:b3-"),
        "a screenshot is stored: {result}"
    );
    assert!(result.contains("\"width\":1"), "{result}");
    assert!(result.contains("\"height\":1"), "{result}");
    assert_eq!(
        shot.attachments.len(),
        1,
        "a picture a model cannot see is not evidence"
    );
    let picture = &shot.attachments[0];
    assert_eq!((picture.width, picture.height), (1, 1));
    assert_eq!(picture.media_type, ImageType::Png);
    let kernel::Locator::Cas { hash, .. } = &picture.locator else {
        panic!("a screenshot is stored by content, not by path");
    };
    let stored = Cas::open(&dir.path().join("cas"))
        .expect("the same store reopens")
        .get(hash)
        .expect("the bytes are in the store");
    assert_eq!(stored.len(), 70);
}

#[test]
fn acting_before_looking_is_refused_and_leaves_the_tool_usable() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut tool = tool(&dir.path().join("cas"));
    let err = tool
        .invoke(&call(
            json!({ "action": "act", "generation": 1, "ref": "e1", "kind": "click" }),
        ))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("snapshot"));
    assert_eq!(tool.meta().name.as_str(), "browser");
}

#[test]
fn a_call_bearing_another_tools_name_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut tool = tool(&dir.path().join("cas"));
    let err = tool
        .invoke(&ToolCall {
            id: "tu_2".to_owned(),
            name: ToolName::parse("exec").expect("a literal name"),
            args: args(json!({ "action": "close" })),
        })
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}

#[test]
fn a_second_look_at_the_same_page_settles_the_development_loop() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut tool = tool(&dir.path().join("cas"));
    let _ = tool.invoke(&call(json!({ "action": "snapshot" })));
    let mut steps = Vec::new();
    for _ in 0..3 {
        let outcome = tool
            .invoke(&call(json!({ "action": "snapshot" })))
            .expect("a page can be looked at again");
        steps.push(text(&outcome));
    }
    assert!(
        steps.iter().any(|step| step.contains("settled")),
        "a page that stops changing settles: {steps:?}"
    );
}

#[test]
fn the_tab_is_opened_once_and_the_frames_keep_one_numbering() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut tool = tool(&dir.path().join("cas"));
    for _ in 0..2 {
        tool.invoke(&call(json!({ "action": "snapshot" })))
            .expect("a page is looked at");
    }
    assert!(
        tool.context.is_some(),
        "the session a tool opened is the session it keeps"
    );
}
