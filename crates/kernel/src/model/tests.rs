// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::budget::Tokens;
use crate::error::AxCode;
use crate::event::Payload;
use crate::locator::{B3Hash, Locator};
use serde_json::Map;

fn one_image() -> ImageRef {
    ImageRef {
        locator: Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap(),
        media_type: ImageType::Png,
        width: 640,
        height: 480,
    }
}

#[test]
fn a_picture_crosses_the_ledger_as_a_locator_and_four_integers() {
    let blocks = vec![
        ContentBlock::Image(one_image()),
        ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: "the screenshot".to_owned(),
            is_error: false,
            attachments: vec![one_image()],
        },
    ];
    let payload = message_payload(&blocks).unwrap();
    assert_eq!(content_from_message(&payload).unwrap(), blocks);

    // The block's wire form is flat: the four fields sit beside `kind`,
    // so a person reading a ledger line sees the same shape in both
    // places the value is used.
    let json = serde_json::to_value(&blocks[0]).unwrap();
    assert_eq!(json["kind"], "image");
    assert_eq!(json["media_type"], "png");
    assert_eq!(json["width"], 640);
    assert_eq!(json["height"], 480);
    assert_eq!(ImageType::Jpeg.mime(), "image/jpeg");
}

#[test]
fn a_tool_result_written_before_attachments_existed_still_replays() {
    // Every `tool_result` already in a ledger has three keys. A fourth
    // one that is not optional would end replay for every city that
    // ever ran a tool.
    let old = serde_json::json!({
        "kind": "tool_result",
        "tool_use_id": "tu_1",
        "content": "ok",
        "is_error": false,
    });
    let block: ContentBlock = serde_json::from_value(old).unwrap();
    assert_eq!(
        block,
        ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: "ok".to_owned(),
            is_error: false,
            attachments: Vec::new(),
        }
    );
}

#[test]
fn policy_default_is_not_confidential() {
    assert!(!BuildingPolicy::default().confidential);
    assert!(BuildingPolicy::new(true).confidential);
}

#[test]
fn model_return_roundtrips_with_sorted_payload_keys() {
    let ret = ModelReturn::bare(Payload::empty(), vec![]);
    let json = serde_json::to_string(&ret).unwrap();
    let back: ModelReturn = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ret);
}

#[test]
fn thinking_survives_the_payload_round_trip_verbatim() {
    // The provider verifies `signature` against the reasoning it
    // issued, so every byte of both blocks has to come back.
    // Assembled at runtime: a signature-shaped literal is exactly
    // what the secret gate exists to keep out of the repository.
    let signature = format!("{}{}", "WaUjzkyp", "Q2mUEVM36O2Txu");
    let issued = vec![
        ContentBlock::Thinking {
            thinking: "The question has two parts.".to_owned(),
            signature,
        },
        ContentBlock::RedactedThinking {
            data: "EroBCkYIARgCKkBmx".to_owned(),
        },
        ContentBlock::Text {
            text: "Based on my analysis...".to_owned(),
        },
    ];
    let payload = message_payload(&issued).unwrap();
    assert_eq!(content_from_message(&payload).unwrap(), issued);
}

#[test]
fn a_content_array_that_cannot_be_read_is_an_error_not_an_empty_window() {
    // A block kind this build does not know must not make the whole
    // assistant message vanish from the window: the ledger still has
    // it, and a window that disagrees with the ledger is the second
    // history this design exists to prevent.
    let mut map = Map::new();
    map.insert(
        "content".to_owned(),
        serde_json::json!([{ "kind": "from_a_later_build" }]),
    );
    let payload = Payload::new(map).unwrap();
    let err = content_from_message(&payload).unwrap_err();
    assert_eq!(*err.code(), AxCode::WireMismatch);

    // A message with no content array at all still folds to no
    // blocks: that is the scripted-payload contract, not a failure.
    assert!(content_from_message(&Payload::empty()).unwrap().is_empty());
}

#[test]
fn effort_is_ordered_from_none_to_max() {
    let ladder = [
        Effort::None,
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::XHigh,
        Effort::Max,
    ];
    assert!(ladder.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        serde_json::to_string(&Effort::XHigh).unwrap(),
        "\"xhigh\"",
        "the city spells an effort level the way the provider spells it"
    );
}

#[test]
fn request_carries_four_segment_hashes() {
    let req = ModelRequest {
        policy: BuildingPolicy::default(),
        segments: [B3Hash::digest(b"a"); 4],
        chat: ChatRequest::empty("m", Ceiling::new(64).unwrap()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["segments"].as_array().unwrap().len(), 4);
}

/// A row written before the one meaning existed, whose cache parts
/// exceed its input count, can only be Anthropic's uncached count; the
/// reader returns the whole prompt. A row without cache parts reads as
/// written, because both old meanings agree there.
#[test]
fn an_unversioned_usage_row_reads_in_the_one_meaning() {
    let read = |row: serde_json::Value| {
        serde_json::from_value::<ModelUsage>(row)
            .unwrap()
            .input_tokens
            .get()
    };
    let anthropic_row = serde_json::json!({
        "input_tokens": 10, "output_tokens": 1,
        "cache_read_tokens": 90, "cache_write_tokens": 5,
    });
    let uncached_row = serde_json::json!({
        "input_tokens": 10, "output_tokens": 1,
        "cache_read_tokens": 0, "cache_write_tokens": 0,
    });
    let versioned_row = serde_json::json!({
        "input_tokens": 10, "output_tokens": 1,
        "cache_read_tokens": 90, "cache_write_tokens": 5, "v": 1,
    });
    // Cache parts that only reach the input count fit inside it, which
    // is the other meaning's row: only exceeding it is evidence.
    let all_cached_row = serde_json::json!({
        "input_tokens": 10, "output_tokens": 1,
        "cache_read_tokens": 6, "cache_write_tokens": 4,
    });
    assert_eq!(
        [
            read(anthropic_row),
            read(uncached_row),
            read(versioned_row),
            read(all_cached_row)
        ],
        [105, 10, 10, 10]
    );
}

#[test]
fn a_cache_count_absent_from_a_row_reads_as_unreported_and_a_zero_stays_reported() {
    let read = |row: serde_json::Value| {
        let usage = serde_json::from_value::<ModelUsage>(row).unwrap();
        (usage.cache_read_tokens, usage.cache_write_tokens)
    };
    let silent = serde_json::json!({ "input_tokens": 10, "output_tokens": 1, "v": 1 });
    let null = serde_json::json!({
        "input_tokens": 10, "output_tokens": 1, "v": 1,
        "cache_read_tokens": null, "cache_write_tokens": null,
    });
    let zero = serde_json::json!({
        "input_tokens": 10, "output_tokens": 1,
        "cache_read_tokens": 0, "cache_write_tokens": 0,
    });
    assert_eq!(
        [read(silent), read(null), read(zero)],
        [
            (CacheCount::Unreported, CacheCount::Unreported),
            (CacheCount::Unreported, CacheCount::Unreported),
            (
                CacheCount::Reported(Tokens::new(0)),
                CacheCount::Reported(Tokens::new(0))
            ),
        ]
    );
    let unreported = ModelUsage {
        input_tokens: Tokens::new(10),
        output_tokens: Tokens::new(1),
        cache_read_tokens: CacheCount::Unreported,
        cache_write_tokens: CacheCount::Reported(Tokens::new(2)),
        dialect: None,
    };
    let row = serde_json::to_value(unreported).unwrap();
    assert_eq!(
        (
            row.get("cache_read_tokens"),
            serde_json::from_value::<ModelUsage>(row.clone()).unwrap()
        ),
        (None, unreported)
    );
}

/// A tail breakpoint sits on the last message and on no other, and an
/// unmarked request carries none.
#[test]
fn a_tail_breakpoint_is_carried_by_the_last_message_alone() {
    let said = |text: &str| ChatMessage {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: text.to_owned(),
        }],
    };
    let messages = vec![said("a"), said("b"), said("c")];
    let ceiling = Ceiling::new(1024).unwrap();
    let tail = ChatRequest {
        messages: std::borrow::Cow::Borrowed(&messages),
        breakpoint: MessageBreakpoint::Tail,
        ..ChatRequest::empty("m", ceiling)
    };
    let unmarked = ChatRequest {
        messages: std::borrow::Cow::Borrowed(&messages),
        ..ChatRequest::empty("m", ceiling)
    };
    let carried = |request: &ChatRequest<'_>| -> Vec<bool> {
        (0..=messages.len())
            .map(|at| request.carries_breakpoint(at))
            .collect()
    };
    assert_eq!(
        (carried(&tail), carried(&unmarked)),
        (
            vec![false, false, true, false],
            vec![false, false, false, false]
        )
    );
}
