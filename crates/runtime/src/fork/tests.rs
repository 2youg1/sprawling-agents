// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a branch inherits: the mother's conversation, rebuilt from her
//! own records, cut at the last line that is a place a conversation can
//! be cut.
//!
//! The ledger is built by a real one (`memory::JsonlLedger`), so the
//! bytes these tests read are the bytes a run writes: the point of
//! rebuilding from records is that they survive a process, and a
//! hand-built payload would not prove that.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]

use kernel::event::record::{ModelReturned, RunStarted, ToolAnswer, ToolCalled, ToolResult};
use kernel::{Address, ContentBlock, EventDraft, EventKind, Payload, Role, RunId, Seq, TimeMs};

use crate::replay::{self, VerifiedLedger};

fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

/// One line of the mother's conversation, as the turn layer appends it.
fn line(kind: EventKind, data: Payload) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(0),
        who: "lab/room1".to_owned(),
        addr: Some(room()),
        kind,
        data,
        ig: false,
    }
}

/// A run that made one turn: a task, a reply that asked for a call, the
/// call, and its result.
fn mother() -> VerifiedLedger {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = memory::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    ledger.append_all(mother_drafts()).unwrap();
    replay::verify_ledger_dir(dir.path()).unwrap()
}

/// The mother's four lines, as drafts.
fn mother_drafts() -> Vec<EventDraft> {
    vec![
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
                    text: "reading it now".to_owned(),
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
                    result: Payload::new(
                        [("said".to_owned(), serde_json::Value::from("42"))]
                            .into_iter()
                            .collect(),
                    )
                    .unwrap(),
                },
            })
            .unwrap(),
        ),
    ]
}

/// A mother run with a line of a newer kind inside it: the verified door
/// skips the line, and the indexed door production calls reaches the same
/// rebuild rather than refusing it.
#[test]
fn both_doors_rebuild_a_mother_run_holding_a_line_of_a_newer_kind() {
    let dir = tempfile::tempdir().unwrap();
    let drafts = mother_drafts();
    let (head, tail) = drafts.split_at(2);
    {
        let (mut ledger, _) = memory::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.append_all(head.to_vec()).unwrap();
    }
    let segment = memory::ledger_segments_at(dir.path())
        .unwrap()
        .pop()
        .unwrap();
    let mut written = std::fs::read(&segment).unwrap();
    let last = written
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .last()
        .unwrap()
        .to_vec();
    let future = format!(
        "{{\"v\":1,\"run\":{},\"seq\":2,\"prev\":\"{}\",\"t\":0,\"who\":\"city\",\"kind\":\"kind_from_the_future\",\"data\":{{}},\"ig\":true}}\n",
        serde_json::to_string(&RunId::CITY).unwrap(),
        kernel::ledger::chain_hash(&last)
    );
    written.extend_from_slice(future.as_bytes());
    std::fs::write(&segment, written).unwrap();
    {
        let (mut ledger, _) = memory::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.append_all(tail.to_vec()).unwrap();
    }
    let verified = replay::verify_ledger_dir(dir.path()).unwrap();
    let at = verified.tail_seq().unwrap();
    let mut index = memory::LedgerIndex::empty();
    index.refresh(dir.path()).unwrap();
    assert_eq!(
        super::inherited_indexed(&index, dir.path(), at).unwrap(),
        super::inherited(&verified, at).unwrap()
    );
}

/// The rebuild is the mother's own window, folded through the same type
/// the live loop folds through.
#[test]
fn a_branch_inherits_the_mother_window_message_for_message() {
    let verified = mother();
    let inherited = crate::fork::inherited(&verified, Seq::new(3)).unwrap();
    assert_eq!(inherited.at, Seq::new(3));

    let mut expected = crate::conversation::Conversation::new();
    expected.push_task_lines(
        "measure the meter",
        "a number is written down",
        crate::conversation::Opening::WithPerson,
    );
    expected.push_assistant(vec![ContentBlock::Text {
        text: "reading it now".to_owned(),
    }]);
    expected.push_tool_results(vec![ContentBlock::ToolResult {
        tool_use_id: "tu_1".to_owned(),
        content: "{\"said\":\"42\"}".to_owned(),
        is_error: false,
        attachments: Vec::new(),
    }]);
    assert_eq!(inherited.messages, expected.messages().to_vec());
    assert_eq!(inherited.messages[0].role, Role::User);
    assert_eq!(inherited.messages[1].role, Role::Assistant);
    assert_eq!(inherited.messages[2].role, Role::User);
}

/// A cut inside a wave of calls moves back to the last line that is a
/// place a conversation can be cut: an assistant message whose tool uses
/// nothing answers is a shape no provider accepts.
#[test]
fn a_cut_inside_a_wave_moves_back_to_the_safe_point() {
    let verified = mother();

    // Line 2 is the call and line 3 its result, so a branch at the call
    // itself ends at the reply that asked for it - and the reply is
    // dropped too, because a call with no answer is half an exchange.
    let at_call = crate::fork::inherited(&verified, Seq::new(2)).unwrap();
    assert_eq!(at_call.at, Seq::new(0));
    assert!(
        at_call
            .messages
            .iter()
            .all(|message| message.role != Role::Assistant),
        "the half-answered wave is dropped whole: {:?}",
        at_call.messages
    );

    // A branch at the run's first line inherits the task and nothing
    // else, which is a conversation one message old.
    let at_open = crate::fork::inherited(&verified, Seq::new(0)).unwrap();
    assert_eq!(at_open.at, Seq::new(0));
    assert_eq!(at_open.messages.len(), 1);
}

/// A line the history does not hold is refused with the fact that
/// settles it: where the sequence ends.
#[test]
fn a_cut_the_history_does_not_hold_is_refused() {
    let verified = mother();
    let err = crate::fork::inherited(&verified, Seq::new(99)).unwrap_err();
    assert_eq!(err.code(), &kernel::AxCode::InvalidArgs);
    assert!(err.recovery().contains("ends at seq 3"), "{err}");
}

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
    let verified = replay::verify_ledger_dir(dir.path()).unwrap();
    let inherited = crate::fork::inherited(&verified, Seq::new(3)).unwrap();

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
