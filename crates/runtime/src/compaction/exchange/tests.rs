// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{ContentBlock, Payload};

use super::Exchange;

fn text(of: &str) -> ContentBlock {
    ContentBlock::Text {
        text: of.to_owned(),
    }
}

fn result(of: &str) -> ContentBlock {
    ContentBlock::ToolResult {
        tool_use_id: "call-1".to_owned(),
        content: of.to_owned(),
        is_error: false,
        attachments: Vec::new(),
    }
}

/// Deterministic prose of at least `len` bytes, cut on a word end.
fn prose(len: usize) -> String {
    let unit = "lorem ipsum dolor sit amet ";
    let mut out = unit.repeat(len / unit.len() + 1);
    out.truncate(len);
    out
}

/// What a tool result's text looks like: one printed JSON payload.
fn printed(note: usize) -> String {
    let payload = Payload::of(&serde_json::json!({ "note": prose(note) })).unwrap();
    serde_json::to_string(&payload).unwrap()
}

#[test]
fn an_exchange_within_its_budget_keeps_every_text() {
    let small = prose(200);
    let mut exchange = Exchange::new();
    exchange.push_assistant(vec![text(&small)]);
    exchange.push_result(result(&printed(300)));
    let before = exchange.clone();
    exchange.compact().unwrap();
    assert_eq!(exchange, before);
}

#[test]
fn an_over_budget_reply_is_shortened_and_a_structured_result_stays_whole() {
    let reply = prose(60_000);
    let answer = printed(40_000);
    let mut exchange = Exchange::new();
    exchange.push_assistant(vec![text(&reply)]);
    exchange.push_result(result(&answer));
    exchange.compact().unwrap();
    let compacted = match &exchange.assistant()[0] {
        ContentBlock::Text { text } => text.clone(),
        other => panic!("expected a text block, got {other:?}"),
    };
    assert!(!compacted.is_empty());
    assert!(compacted.len() < reply.len(), "the reply was shortened");
    // Structured data is never cut: a half JSON body reads parseable and
    // is not, so `plan` sends it out whole or keeps it whole.
    match &exchange.results()[0] {
        ContentBlock::ToolResult { content, .. } => assert_eq!(content, &answer),
        other => panic!("expected a tool_result block, got {other:?}"),
    }
}

#[test]
fn the_share_follows_the_size_of_the_whole_exchange() {
    // The same reply, compacted in two differently sized exchanges,
    // comes out two lengths: the share is the budget over the texts the
    // whole exchange holds. This difference is what a mid-wave
    // compaction would freeze in, and what the turn boundary refuses to.
    let reply = prose(40_000);
    let mut alone = Exchange::new();
    alone.push_assistant(vec![text(&reply)]);
    let mut with_wave = Exchange::new();
    with_wave.push_assistant(vec![text(&reply)]);
    with_wave.push_result(result(&printed(40_000)));
    alone.compact().unwrap();
    with_wave.compact().unwrap();
    let shortened = |exchange: &Exchange| match &exchange.assistant()[0] {
        ContentBlock::Text { text } => text.len(),
        other => panic!("expected a text block, got {other:?}"),
    };
    assert!(shortened(&alone) > shortened(&with_wave));
}
