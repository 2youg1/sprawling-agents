// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Each payload against its expected map, for both endings of a call,
//! with the call's subject decided on the way in.

use serde_json::{Map, Value};

use super::*;
use crate::error::{AxCode, AxError};
use crate::event::Payload;

fn bytes(payload: &Payload) -> String {
    serde_json::to_string(payload).unwrap()
}

fn read_args() -> Payload {
    let mut args = Map::new();
    args.insert("path".to_owned(), Value::String("lobby/City.md".to_owned()));
    args.insert("limit".to_owned(), Value::Number(200u64.into()));
    Payload::new(args).unwrap()
}

fn read_result() -> Payload {
    let mut out = Map::new();
    out.insert("text".to_owned(), Value::String("one line".to_owned()));
    out.insert("lines".to_owned(), Value::Number(1u64.into()));
    Payload::new(out).unwrap()
}

fn refusal() -> AxError {
    AxError::failure(AxCode::PathNotFound, "read a file", "lobby/gone.md")
        .with_recovery("read a path this building holds")
}

#[test]
fn a_call_line_writes_the_bytes_its_map_declares() {
    let args = read_args();
    let mut hand = Map::new();
    hand.insert("id".to_owned(), Value::String("call_1".to_owned()));
    hand.insert("name".to_owned(), Value::String("read".to_owned()));
    hand.insert("args".to_owned(), serde_json::to_value(&args).unwrap());
    hand.insert(
        "subject".to_owned(),
        Value::String("lobby/City.md".to_owned()),
    );
    let hand = Payload::new(hand).unwrap();

    let subject = ToolCalled::subject_of(&args);
    let called = ToolCalled {
        id: "call_1".to_owned(),
        name: ToolName::parse("read").unwrap(),
        args,
        subject,
    };
    let typed = Payload::of(&called).unwrap();
    assert_eq!(bytes(&typed), bytes(&hand));
    assert_eq!(typed.read::<ToolCalled>().unwrap(), called);
}

#[test]
fn the_subject_prefers_the_key_the_tools_name_first() {
    let args = read_args();
    assert_eq!(
        ToolCalled::subject_of(&args).as_deref(),
        Some("lobby/City.md")
    );
}

#[test]
fn a_tool_with_no_preferred_key_is_still_named_by_what_it_acted_on() {
    let mut args = Map::new();
    args.insert("body".to_owned(), Value::String("ship it".to_owned()));
    let args = Payload::new(args).unwrap();
    assert_eq!(ToolCalled::subject_of(&args).as_deref(), Some("ship it"));
}

/// The fallback takes the payload's own key order, so two machines read
/// one record the same way. This is the pair that used to be read as
/// `"a"` by the city and `"b"` by a browser.
#[test]
fn a_subject_the_arguments_do_not_name_is_the_first_in_key_order() {
    let mut args = Map::new();
    args.insert("10".to_owned(), Value::String("a".to_owned()));
    args.insert("2".to_owned(), Value::String("b".to_owned()));
    let args = Payload::new(args).unwrap();
    assert_eq!(ToolCalled::subject_of(&args).as_deref(), Some("a"));
}

#[test]
fn a_call_whose_arguments_name_nothing_has_no_subject() {
    let mut args = Map::new();
    args.insert("shape".to_owned(), Value::Number(3u64.into()));
    let args = Payload::new(args).unwrap();
    assert_eq!(ToolCalled::subject_of(&args), None);
}

#[test]
fn a_call_line_written_before_the_subject_existed_still_reads() {
    let mut hand = Map::new();
    hand.insert("id".to_owned(), Value::String("call_1".to_owned()));
    hand.insert("name".to_owned(), Value::String("read".to_owned()));
    hand.insert(
        "args".to_owned(),
        serde_json::to_value(read_args()).unwrap(),
    );
    let old = Payload::new(hand).unwrap();
    assert_eq!(old.read::<ToolCalled>().unwrap().subject, None);
}

#[test]
fn a_call_that_answered_writes_result_and_no_error() {
    let result = read_result();
    let mut hand = Map::new();
    hand.insert("tool_use_id".to_owned(), Value::String("call_1".to_owned()));
    hand.insert("name".to_owned(), Value::String("read".to_owned()));
    hand.insert("result".to_owned(), serde_json::to_value(&result).unwrap());
    let hand = Payload::new(hand).unwrap();

    let answered = ToolResult {
        tool_use_id: "call_1".to_owned(),
        name: ToolName::parse("read").unwrap(),
        answer: ToolAnswer::Answered { result },
    };
    let typed = Payload::of(&answered).unwrap();
    assert_eq!(bytes(&typed), bytes(&hand));
    assert_eq!(typed.read::<ToolResult>().unwrap(), answered);
}

#[test]
fn a_call_that_failed_writes_error_and_no_result() {
    let err = refusal();
    let mut hand = Map::new();
    hand.insert("tool_use_id".to_owned(), Value::String("call_2".to_owned()));
    hand.insert("name".to_owned(), Value::String("read".to_owned()));
    hand.insert("error".to_owned(), serde_json::to_value(&err).unwrap());
    let hand = Payload::new(hand).unwrap();

    let failed = ToolResult {
        tool_use_id: "call_2".to_owned(),
        name: ToolName::parse("read").unwrap(),
        answer: ToolAnswer::Failed {
            error: Payload::of(&err).unwrap(),
        },
    };
    let typed = Payload::of(&failed).unwrap();
    assert_eq!(bytes(&typed), bytes(&hand));
    assert_eq!(typed.read::<ToolResult>().unwrap(), failed);
}
