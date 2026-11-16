// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a branch inherits: the mother's conversation, rebuilt from her
//! own records, cut at the last line that is a place a conversation can
//! be cut.
//!
//! The ledger is built by a real one (`storage::JsonlLedger`), so the
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

use kernel::event::record::{
    ModelReturned, RunStarted, SteerReceived, ToolAnswer, ToolCalled, ToolResult,
};
use kernel::{Address, ContentBlock, EventDraft, EventKind, Payload, Role, RunId, Seq, TimeMs};

use super::Inherited;

fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

/// One line of the mother's conversation, as the turn layer appends it.
pub(super) fn line(kind: EventKind, data: Payload) -> EventDraft {
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
fn mother() -> Mother {
    verified(mother_drafts())
}

/// A ledger on disk holding `drafts`, and its index.
fn verified(drafts: Vec<EventDraft>) -> Mother {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = storage::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    ledger.append_all(drafts).unwrap();
    drop(ledger);
    Mother::written(dir)
}

/// A mother's ledger on disk and the index its only writer keeps over it.
pub(super) struct Mother {
    dir: tempfile::TempDir,
    index: storage::LedgerIndex,
}

impl Mother {
    pub(super) fn written(dir: tempfile::TempDir) -> Mother {
        let mut index = storage::LedgerIndex::empty();
        index.refresh(dir.path()).unwrap();
        Mother { dir, index }
    }

    pub(super) fn inherited(&self, at: Seq) -> Result<Inherited, kernel::AxError> {
        super::inherited_indexed(&self.index, self.dir.path(), at)
    }
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
                first_at: None,
                first_us: None,
                took_us: None,
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
                effect: None,
                render: None,
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
                took_us: None,
                rendered: None,
                shown_bytes: None,
                attachments: Vec::new(),
            })
            .unwrap(),
        ),
    ]
}

fn read_result(id: &str, rooms: &[&str]) -> EventDraft {
    read_result_with_text(id, rooms, "fn main() {}\n")
}

fn read_result_with_text(id: &str, rooms: &[&str], text: &str) -> EventDraft {
    let rules: Vec<serde_json::Value> = rooms
        .iter()
        .map(|room| serde_json::json!({"room": room, "text": "Use LF.\n"}))
        .collect();
    line(
        EventKind::ToolResult,
        Payload::of(&ToolResult {
            tool_use_id: id.to_owned(),
            name: kernel::ToolName::parse("read").unwrap(),
            answer: ToolAnswer::Answered {
                result: Payload::of(&serde_json::json!({
                    "path": "lab/pkg/main.rs", "bytes": text.len(), "text": text, "rules": rules,
                }))
                .unwrap(),
            },
            took_us: None,
            rendered: Some(kernel::event::record::Rendering::V4),
            shown_bytes: None,
            attachments: Vec::new(),
        })
        .unwrap(),
    )
}

fn returned(calls: u64) -> EventDraft {
    line(
        EventKind::ModelReturned,
        Payload::of(&ModelReturned {
            message: kernel::model::message_payload(&[ContentBlock::Text {
                text: "reading".to_owned(),
            }])
            .unwrap(),
            calls,
            usage: None,
            stop: None,
            billed_usd_micros: None,
            first_at: None,
            first_us: None,
            took_us: None,
        })
        .unwrap(),
    )
}

mod cut;
mod rules;
mod window;
