// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes these structs write are the bytes the hand-written maps in
//! `runtime::prefix` and `runtime::turn` wrote. Each test keeps the old
//! `insert` sequence verbatim and asserts the two encodings are the same
//! string, so a replay over a ledger written by the previous build reads
//! back and re-hashes identically.

use serde_json::{Map, Value, json};

use super::*;
use crate::budget::Tokens;
use crate::event::Payload;

fn bytes(payload: &Payload) -> String {
    serde_json::to_string(payload).unwrap()
}

fn city() -> Address {
    Address::parse("City.md").unwrap()
}

fn rules() -> Address {
    Address::parse("lobby/RULES.md").unwrap()
}

/// The writer that stood in `runtime::prefix::FrozenPrefix::prompt_payload`
/// before this struct existed, copied key for key.
fn hand_written_prompt(hash: &B3Hash) -> Payload {
    let mut entry = Map::new();
    entry.insert("slot".to_owned(), Value::String("city".to_owned()));
    entry.insert("hash".to_owned(), Value::String(hash.to_string()));
    entry.insert("len".to_owned(), Value::Number(41u64.into()));
    entry.insert(
        "sources".to_owned(),
        Value::Array(vec![
            json!({ "addr": "City.md", "kept": 12, "marker": true, "dropped": 4 }),
            json!({ "addr": "lobby/RULES.md", "kept": 25, "marker": false, "dropped": 0 }),
        ]),
    );
    entry.insert(
        "skipped".to_owned(),
        Value::Array(vec![json!({ "addr": "City.md", "reason": "duplicate" })]),
    );
    let mut map = Map::new();
    map.insert(
        "segments".to_owned(),
        Value::Array(vec![Value::Object(entry)]),
    );
    map.insert(
        "breakpoints".to_owned(),
        json!(["city", "building", "resident", "run"]),
    );
    Payload::new(map).unwrap()
}

#[test]
fn a_prompt_line_writes_the_bytes_the_hand_written_map_wrote() {
    let hash = B3Hash::digest(b"the four slots, joined");
    let assembled = PromptAssembled {
        segments: vec![PromptSegment {
            slot: "city".to_owned(),
            hash,
            len: 41,
            sources: vec![
                PromptSource {
                    addr: city(),
                    kept: 12,
                    marker: true,
                    dropped: 4,
                },
                PromptSource {
                    addr: rules(),
                    kept: 25,
                    marker: false,
                    dropped: 0,
                },
            ],
            skipped: vec![PromptSkip {
                addr: city(),
                reason: SkipReason::Duplicate,
            }],
        }],
        breakpoints: ["city", "building", "resident", "run"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
    };
    let typed = Payload::of(&assembled).unwrap();
    assert_eq!(bytes(&typed), bytes(&hand_written_prompt(&hash)));
    assert_eq!(typed.read::<PromptAssembled>().unwrap(), assembled);
}

/// The four spellings a rebuild and a page read a skip under. A fifth
/// reason has to be added here before it can be written.
#[test]
fn every_skip_reason_keeps_the_word_the_assembler_wrote() {
    let spellings: Vec<String> = [
        SkipReason::Duplicate,
        SkipReason::Unreadable,
        SkipReason::NotUtf8,
        SkipReason::NoBudget,
    ]
    .into_iter()
    .map(|reason| serde_json::to_string(&reason).unwrap())
    .collect();
    assert_eq!(
        spellings,
        vec![
            "\"duplicate\"",
            "\"unreadable\"",
            "\"not_utf8\"",
            "\"no_budget\""
        ]
    );
}

/// The writer that stood in `runtime::turn::Turn::call`, copied key for
/// key.
fn hand_written_called(segments: &[B3Hash; 2]) -> Payload {
    let mut called = Map::new();
    called.insert(
        "segments".to_owned(),
        Value::Array(
            segments
                .iter()
                .map(|hash| Value::String(hash.to_string()))
                .collect(),
        ),
    );
    called.insert("model".to_owned(), Value::String("claude-x".to_owned()));
    Payload::new(called).unwrap()
}

#[test]
fn a_call_line_writes_the_bytes_the_hand_written_map_wrote() {
    let segments = [B3Hash::digest(b"city"), B3Hash::digest(b"run")];
    let called = ModelCalled {
        segments: segments.to_vec(),
        model: "claude-x".to_owned(),
    };
    let typed = Payload::of(&called).unwrap();
    assert_eq!(bytes(&typed), bytes(&hand_written_called(&segments)));
    assert_eq!(typed.read::<ModelCalled>().unwrap(), called);
}

/// The writer that stood in `runtime::turn::Turn::call` for the reply,
/// copied key for key, including which keys it left out.
fn hand_written_returned(
    message: &Payload,
    usage: Option<ModelUsage>,
    stop: Option<StopReason>,
    billed: Option<UsdMicros>,
) -> Payload {
    let mut returned = Map::new();
    returned.insert("message".to_owned(), serde_json::to_value(message).unwrap());
    returned.insert("calls".to_owned(), Value::Number(2u64.into()));
    if let Some(usage) = &usage {
        returned.insert("usage".to_owned(), serde_json::to_value(usage).unwrap());
    }
    if let Some(stop) = &stop {
        returned.insert("stop".to_owned(), serde_json::to_value(stop).unwrap());
    }
    if let Some(billed) = billed {
        returned.insert(
            "billed_usd_micros".to_owned(),
            Value::Number(billed.get().into()),
        );
    }
    Payload::new(returned).unwrap()
}

fn assistant_message() -> Payload {
    let mut map = Map::new();
    map.insert("role".to_owned(), Value::String("assistant".to_owned()));
    map.insert(
        "content".to_owned(),
        json!([{ "kind": "text", "text": "reading the file now" }]),
    );
    Payload::new(map).unwrap()
}

#[test]
fn a_reply_line_writes_the_bytes_the_hand_written_map_wrote() {
    let message = assistant_message();
    let usage = ModelUsage {
        input_tokens: Tokens::new(1200),
        output_tokens: Tokens::new(64),
        cache_read_tokens: Tokens::new(0),
        cache_write_tokens: Tokens::new(0),
    };
    let returned = ModelReturned {
        message: message.clone(),
        calls: 2,
        usage: Some(usage),
        stop: Some(StopReason::ToolUse),
        billed_usd_micros: Some(UsdMicros::new(3400)),
    };
    let typed = Payload::of(&returned).unwrap();
    assert_eq!(
        bytes(&typed),
        bytes(&hand_written_returned(
            &message,
            Some(usage),
            Some(StopReason::ToolUse),
            Some(UsdMicros::new(3400))
        ))
    );
    assert_eq!(typed.read::<ModelReturned>().unwrap(), returned);
}

/// A provider that reported nothing but the reply leaves three keys
/// absent, exactly as the hand-written writer did: absent is not zero.
#[test]
fn a_reply_that_reported_nothing_leaves_the_three_keys_absent() {
    let message = assistant_message();
    let returned = ModelReturned {
        message: message.clone(),
        calls: 2,
        usage: None,
        stop: None,
        billed_usd_micros: None,
    };
    let typed = Payload::of(&returned).unwrap();
    assert_eq!(
        bytes(&typed),
        bytes(&hand_written_returned(&message, None, None, None))
    );
    assert_eq!(typed.as_map().len(), 2);
}

#[test]
fn a_steer_line_writes_the_bytes_the_hand_written_map_wrote() {
    let mut map = Map::new();
    map.insert("source".to_owned(), Value::String("person".to_owned()));
    map.insert(
        "text".to_owned(),
        Value::String("try the other file".to_owned()),
    );
    let hand = Payload::new(map).unwrap();
    let steer = SteerReceived {
        source: "person".to_owned(),
        text: "try the other file".to_owned(),
    };
    let typed = Payload::of(&steer).unwrap();
    assert_eq!(bytes(&typed), bytes(&hand));
    assert_eq!(typed.read::<SteerReceived>().unwrap(), steer);
}
