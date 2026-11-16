// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use serde_json::{Value, json};

fn page(generation: u64) -> PageSnapshot {
    PageSnapshot::read(
        generation,
        &json!([
            { "role": "button", "name": "Place order" },
            { "role": "textbox", "name": "Quantity" },
        ]),
    )
    .unwrap()
}

fn expression(frame: &Frame) -> String {
    frame
        .params()
        .get("expression")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn a_click_names_the_node_the_snapshot_showed() {
    let mut session = Session::new();
    let context = ContextId::parse("c1").unwrap();
    let frame = frame_for(
        &mut session,
        &context,
        &page(1),
        1,
        &Action::Click {
            reference: "e1".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(frame.method(), "script.evaluate");
    assert!(expression(&frame).contains("'button'"));
    assert!(expression(&frame).ends_with(".click()"));
}

#[test]
fn an_action_decided_against_an_older_page_is_refused_not_retried() {
    let mut session = Session::new();
    let context = ContextId::parse("c1").unwrap();
    let err = frame_for(
        &mut session,
        &context,
        &page(4),
        3,
        &Action::Click {
            reference: "e1".to_owned(),
        },
    )
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("fresh snapshot"));
}

#[test]
fn text_a_page_could_choose_never_becomes_code() {
    let mut session = Session::new();
    let context = ContextId::parse("c1").unwrap();
    let frame = frame_for(
        &mut session,
        &context,
        &page(1),
        1,
        &Action::Type {
            reference: "e2".to_owned(),
            text: "'); fetch('https://elsewhere.test'); ('".to_owned(),
        },
    )
    .unwrap();
    let script = expression(&frame);
    assert!(
        script.contains("\\'"),
        "the quote that would close the literal carries a backslash: {script}"
    );
    // The page's text sits inside exactly one literal: two
    // delimiters, and every quote between them escaped. Counting is
    // the assertion because "looks escaped" is not a property.
    let after = script.split("el.value = ").nth(1).unwrap();
    let literal = after.split("; el.dispatchEvent").next().unwrap();
    let mut bare = 0usize;
    let mut escaped = false;
    for ch in literal.chars() {
        match (escaped, ch) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '\'') => bare += 1,
            (false, _) => {}
        }
    }
    assert_eq!(
        bare, 2,
        "only the delimiters may be unescaped quotes: {literal}"
    );
}

#[test]
fn a_reference_that_was_never_minted_is_refused_by_name() {
    let mut session = Session::new();
    let context = ContextId::parse("c1").unwrap();
    for reference in ["e9", "button", "e0", "e"] {
        assert!(
            frame_for(
                &mut session,
                &context,
                &page(1),
                1,
                &Action::Read {
                    reference: reference.to_owned()
                },
            )
            .is_err(),
            "{reference}"
        );
    }
}
