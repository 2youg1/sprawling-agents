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
    ];
    ledger.append_all(drafts.to_vec()).unwrap();
    replay::verify_ledger_dir(dir.path()).unwrap()
}

/// The rebuild is the mother's own window, folded through the same type
/// the live loop folds through.
#[test]
fn a_branch_inherits_the_mother_window_message_for_message() {
    let verified = mother();
    let inherited = crate::fork::inherited(&verified, Seq::new(3)).unwrap();
    assert_eq!(inherited.at, Seq::new(3));

    let mut expected = crate::window::Window::new();
    expected.push_task_lines(
        "measure the meter",
        "a number is written down",
        crate::window::Opening::WithPerson,
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
