// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a branch of a loud mother inherits: the exchange the turn
//! boundary compacted, rebuilt from the same records the mother wrote.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]

use kernel::event::record::{ModelReturned, RunStarted, ToolAnswer, ToolCalled, ToolResult};
use kernel::{ContentBlock, EventKind, Payload, Seq, TimeMs};

use super::tests::{Mother, line};

/// A loud mother: a reply and a result whose texts overflow the
/// exchange's budget together. What a branch inherits is the compacted
/// exchange — the same bytes the turn boundary folded into the mother's
/// window, computed at the same boundary of the same turn.
#[test]
fn a_branch_inherits_the_compacted_exchange_not_the_raw_records() {
    fn prose(len: usize) -> String {
        let unit = "lorem ipsum dolor sit amet ";
        let mut out = unit.repeat(len / unit.len() + 1);
        out.truncate(len);
        out
    }
    let reply = prose(40_000);
    let answer =
        serde_json::to_string(&Payload::of(&serde_json::json!({ "note": prose(20_000) })).unwrap())
            .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = memory::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    let drafts = [
        line(
            EventKind::RunStarted,
            Payload::of(&RunStarted {
                task: "measure the meter".to_owned(),
                goal: "a number is written down".to_owned(),
                ..RunStarted::default()
            })
            .unwrap(),
        ),
        line(
            EventKind::ModelReturned,
            Payload::of(&ModelReturned {
                message: kernel::model::message_payload(&[ContentBlock::Text {
                    text: reply.clone(),
                }])
                .unwrap(),
                calls: 1,
                usage: None,
                stop: None,
                billed_usd_micros: None,
            })
            .unwrap(),
        ),
        line(
            EventKind::ToolCalled,
            Payload::of(&ToolCalled {
                id: "tu_1".to_owned(),
                name: kernel::ToolName::parse("status").unwrap(),
                args: Payload::empty(),
                subject: None,
            })
            .unwrap(),
        ),
        line(
            EventKind::ToolResult,
            Payload::of(&ToolResult {
                tool_use_id: "tu_1".to_owned(),
                name: kernel::ToolName::parse("status").unwrap(),
                answer: ToolAnswer::Answered {
                    result: Payload::of(&serde_json::json!({ "note": prose(20_000) })).unwrap(),
                },
            })
            .unwrap(),
        ),
    ];
    ledger.append_all(drafts.to_vec()).unwrap();
    drop(ledger);
    let inherited = Mother::written(dir).inherited(Seq::new(3)).unwrap();

    let mut exchange = crate::compaction::Exchange::new();
    exchange.push_assistant(vec![ContentBlock::Text {
        text: reply.clone(),
    }]);
    exchange.push_result(ContentBlock::ToolResult {
        tool_use_id: "tu_1".to_owned(),
        content: answer.clone(),
        is_error: false,
        attachments: Vec::new(),
    });
    exchange.compact().unwrap();

    let assistant = inherited.messages[1].content.clone();
    assert_eq!(assistant, exchange.assistant().to_vec());
    let results = inherited.messages[2].content.clone();
    assert_eq!(results, exchange.results().to_vec());
    // The exchange overflows its budget as one turn, and the reply is
    // what the compaction shortened: the raw record is longer than what
    // a branch starts from.
    assert!(reply.len() > assistant_text(&assistant).len());
}

fn assistant_text(blocks: &[ContentBlock]) -> String {
    match &blocks[0] {
        ContentBlock::Text { text } => text.clone(),
        other => panic!("expected a text block, got {other:?}"),
    }
}
