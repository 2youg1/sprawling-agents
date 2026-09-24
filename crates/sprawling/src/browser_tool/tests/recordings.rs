// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The conversations these tests replay, and the ports that answer them.

use super::*;

pub(super) fn recorded() -> Recording {
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

pub(super) fn tool(cas_at: &std::path::Path) -> BrowserTool {
    let cas = Cas::open(cas_at).expect("a cas opens in a fresh directory");
    BrowserTool::new(Role::Building, Box::new(recorded()), cas).expect("the tool builds")
}

pub(super) fn tool_with(cas_at: &std::path::Path, port: Recording) -> BrowserTool {
    let cas = Cas::open(cas_at).expect("a cas opens in a fresh directory");
    BrowserTool::new(Role::Building, Box::new(port), cas).expect("the tool builds")
}

/// The conversation a screenshot by reference has: the page names the
/// element, and the capture that follows covers it.
pub(super) fn recorded_by_reference() -> Recording {
    let mut mirror = Session::new();
    let context = ContextId::parse("c1").expect("a literal id");
    let mut recording = Recording::new();
    let begin = mirror
        .begin(SessionRequest::default())
        .expect("a session begins");
    recording.answer(&begin, json!({ "sessionId": "s1" }));
    let tree = mirror.tree().expect("the tree is asked for");
    recording.answer(&tree, json!({ "contexts": [{ "context": "c1" }] }));
    let page = json!([{ "role": "button", "name": "Place order" }]);
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
        json!({ "action": "snapshot" }),
        None,
        vec![string_result(&page.to_string())],
    );
    let looked = PageSnapshot::read(1, &page).expect("the tree reads");
    let asked = json!({ "action": "screenshot", "ref": "e1", "generation": 1 });
    answer(
        asked.clone(),
        Some(&looked),
        vec![json!({ "result": { "type": "node", "sharedId": "n1", "value": { "nodeType": 1 } } })],
    );
    // The capture frame is built from the resolve reply, so it is recorded
    // against that same reply.
    let Verb::Screenshot(request) = Verb::read(&args(asked)).expect("a shot reads") else {
        panic!("a screenshot verb");
    };
    let capture = request
        .capture_frame(
            &mut mirror,
            &context,
            &json!({ "result": { "type": "node", "sharedId": "n1" } }),
        )
        .expect("the capture builds");
    recording.answer(&capture, json!({ "data": ONE_RED_PIXEL }));
    recording
}

/// The conversation a screenshot by reference set has: the page reports
/// the boxes, and the capture that follows covers all of them.
pub(super) fn recorded_by_union() -> Recording {
    let mut mirror = Session::new();
    let context = ContextId::parse("c1").expect("a literal id");
    let mut recording = Recording::new();
    let begin = mirror
        .begin(SessionRequest::default())
        .expect("a session begins");
    recording.answer(&begin, json!({ "sessionId": "s1" }));
    let tree = mirror.tree().expect("the tree is asked for");
    recording.answer(&tree, json!({ "contexts": [{ "context": "c1" }] }));
    let page = json!([
        { "role": "button", "name": "Place order" },
        { "role": "textbox", "name": "Quantity" },
    ]);
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
        json!({ "action": "snapshot" }),
        None,
        vec![string_result(&page.to_string())],
    );
    let looked = PageSnapshot::read(1, &page).expect("the tree reads");
    let boxes = json!([
        { "ref": "e1", "x": 10, "y": 20, "width": 100, "height": 50 },
        { "ref": "e2", "x": 150, "y": 5, "width": 40, "height": 200 },
    ]);
    let asked = json!({ "action": "screenshot", "refs": ["e1", "e2"], "generation": 1 });
    answer(
        asked.clone(),
        Some(&looked),
        vec![string_result(&boxes.to_string())],
    );
    let Verb::Screenshot(request) = Verb::read(&args(asked)).expect("a shot reads") else {
        panic!("a screenshot verb");
    };
    let capture = request
        .capture_frame(&mut mirror, &context, &string_result(&boxes.to_string()))
        .expect("the capture builds");
    recording.answer(&capture, json!({ "data": ONE_RED_PIXEL }));
    recording
}
