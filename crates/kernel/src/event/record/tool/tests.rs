// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes these two structs write are the bytes the hand-written
//! maps in `runtime::turn::wave` wrote, for both endings of a call.

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
fn a_call_line_writes_the_bytes_the_hand_written_map_wrote() {
    let args = read_args();
    let mut hand = Map::new();
    hand.insert("id".to_owned(), Value::String("call_1".to_owned()));
    hand.insert("name".to_owned(), Value::String("read".to_owned()));
    hand.insert("args".to_owned(), serde_json::to_value(&args).unwrap());
    let hand = Payload::new(hand).unwrap();

    let called = ToolCalled {
        id: "call_1".to_owned(),
        name: ToolName::parse("read").unwrap(),
        args,
    };
    let typed = Payload::of(&called).unwrap();
    assert_eq!(bytes(&typed), bytes(&hand));
    assert_eq!(typed.read::<ToolCalled>().unwrap(), called);
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
