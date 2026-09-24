// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One conversation with a browser, replayed without one.
//!
//! The recording is the seam's second adapter, so this is the production
//! path with the socket swapped out — not a double of the tool.

use super::*;

mod recordings;

use browser::{BrowserPort, Frame, Recording, Reply, SessionRequest};
use recordings::*;
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

/// A PNG whose header says 2000 by 1500, and nothing else.
///
/// Only the header is read — `Shot::read` takes the two sides from the
/// bytes rather than from what anybody said they would be — so a picture
/// past the cap costs forty-five bytes instead of three megabytes.
const PAST_THE_CAP: &str = concat!(
    "iVBORw0KGgoAAAAN",
    "SUhEUgAAB9AAAAXc",
    "CAYAAAB6ZRUrAAAA",
    "AElFTkSuQmCC",
);

/// A port that answers by arrival rather than by what was asked.
///
/// The recording answers by the frame's own bytes, which is what makes a
/// reordered conversation replay — and it cannot express the one thing
/// the cap needs tested, because the second capture asks the same
/// question as the first and must be answered differently.
#[derive(Default)]
struct Sequence {
    answers: Vec<Value>,
}

impl BrowserPort for Sequence {
    fn send(&mut self, frame: &Frame) -> Result<Reply, AxError> {
        let result = match frame.method() {
            "session.new" => json!({ "sessionId": "s1" }),
            "browsingContext.getTree" => json!({ "contexts": [{ "context": "c1" }] }),
            _ => self.answers.remove(0),
        };
        Ok(Reply::Success {
            id: frame.id(),
            result,
        })
    }
}

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
#[test]
fn a_screenshot_past_the_cap_is_taken_again_and_the_smaller_one_kept() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cas = Cas::open(&dir.path().join("cas")).expect("a cas opens in a fresh directory");
    let port = Sequence {
        answers: vec![
            json!({ "data": PAST_THE_CAP }),
            json!({}),
            json!({ "data": ONE_RED_PIXEL }),
        ],
    };
    let mut tool = BrowserTool::new(Role::Building, Box::new(port), cas).expect("the tool builds");
    let shot = tool
        .invoke(&call(json!({ "action": "screenshot" })))
        .expect("the shot is taken again and kept");
    assert_eq!(shot.attachments.len(), 1);
    assert_eq!(
        (shot.attachments[0].width, shot.attachments[0].height),
        (1, 1)
    );
}

#[test]
fn a_screenshot_by_reference_set_covers_every_element_it_named() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut tool = tool_with(&dir.path().join("cas"), recorded_by_union());
    tool.invoke(&call(json!({ "action": "snapshot" })))
        .expect("the page is looked at");
    let shot = tool
        .invoke(&call(
            json!({ "action": "screenshot", "refs": ["e1", "e2"], "generation": 1 }),
        ))
        .expect("the elements are photographed together");
    let result = text(&shot);
    assert!(
        result.contains("cas:b3-"),
        "a union shot becomes evidence too: {result}"
    );
    assert_eq!(shot.attachments.len(), 1);
}

#[test]
fn a_screenshot_by_reference_covers_the_element_the_page_named() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut tool = tool_with(&dir.path().join("cas"), recorded_by_reference());
    tool.invoke(&call(json!({ "action": "snapshot" })))
        .expect("the page is looked at");
    let shot = tool
        .invoke(&call(
            json!({ "action": "screenshot", "ref": "e1", "generation": 1 }),
        ))
        .expect("the element is photographed");
    let result = text(&shot);
    assert!(
        result.contains("cas:b3-"),
        "a clipped shot becomes evidence too: {result}"
    );
    assert_eq!(shot.attachments.len(), 1);
    assert_eq!(
        (shot.attachments[0].width, shot.attachments[0].height),
        (1, 1)
    );
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
    assert_eq!(picture.media_type, kernel::ImageType::Png);
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

#[test]
fn the_person_tool_asks_before_it_connects_and_names_itself() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cas = Cas::open(&dir.path().join("cas")).expect("a cas opens in a fresh directory");
    let mut waiting = BrowserTool::new(
        Role::PersonWaiting,
        Box::new(crate::browser_bidi::AttachedBrowser::waiting()),
        cas,
    )
    .expect("the tool builds");
    assert_eq!(
        waiting.meta().name.as_str(),
        kernel::ToolName::USER_BROWSER,
        "the person's browser is its own tool"
    );
    assert!(matches!(
        waiting.meta().effect,
        Effect::AttachUserBrowser { address: None }
    ));
    assert!(
        waiting.meta().disclosure.contains("Use `browser`"),
        "the disclosure guides to the tool that needs nobody's permission"
    );
    let ask = ToolCall {
        id: "tu_person".to_owned(),
        name: ToolName::parse(kernel::ToolName::USER_BROWSER).expect("a literal name"),
        args: args(json!({ "action": "snapshot" })),
    };
    let err = waiting
        .invoke(&ask)
        .expect_err("nothing connects without an address");
    assert_eq!(err.code(), &AxCode::BrowserUnavailable);
}

#[test]
fn a_declared_address_is_the_effect_the_attach_door_judges() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cas = Cas::open(&dir.path().join("cas")).expect("a cas opens in a fresh directory");
    let tool = BrowserTool::new(
        Role::PersonAt {
            host: "127.0.0.1".to_owned(),
        },
        Box::new(crate::browser_bidi::AttachedBrowser::at(
            "ws://127.0.0.1:9222/session",
        )),
        cas,
    )
    .expect("the tool builds");
    assert!(matches!(
        tool.meta().effect,
        Effect::AttachUserBrowser { address: Some(ref host) } if host == "127.0.0.1"
    ));
    // The subject is still per call: an open names the page's host, which
    // is the egress half the same door scans.
    let subject = tool
        .subject(&call(
            json!({ "action": "open", "url": "https://example.com/" }),
        ))
        .expect("the call reads");
    assert_eq!(subject, GateSubject::Host("example.com".to_owned()));
    let snapshot = tool
        .subject(&call(json!({ "action": "snapshot" })))
        .expect("the call reads");
    assert_eq!(snapshot, GateSubject::None);
}
