// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code,
    reason = "test code; readings are read through their Debug form in the report"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use kernel::layout::CityLayout;
use kernel::{
    Address, AxError, ContentBlock, EventKind, EventRecord, Model, ModelRequest, ModelReturn, RunId,
};
use serde_json::{Value, json};

use super::ledger::{Lines, read};
use super::roles::{SIBLINGS, calls, says};

pub(super) const NOTES: &str = "notes.md";
pub(super) const NOTES_BEFORE: &str = "before\n";
pub(super) const MAKER: &str = "fire the kiln and offer it for review";
pub(super) const CHECKER: &str = "check the kiln work offered for review";

pub(super) const POTTER: &str = "fire the kiln as the potter";
pub(super) const GLAZER: &str = "fire the kiln as the glazer";
pub(super) const REDO: &str = "the kiln fired, delegated again if it cracks";
pub(super) const CHILD_TASK: &str = "claim node 1 and fire it";
pub(super) const CHILD_GOAL: &str = "node 1 fired";

pub(super) const PLAN: &str = "\
# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | fire the kiln | 1 |  | Not started |  |
";

/// The marks the plan scenarios' runs wait on, and the ledger the lead
/// reads to know a child has frozen.
#[derive(Default)]
pub(super) struct Board {
    marks: Mutex<std::collections::BTreeSet<&'static str>>,
    moved: Condvar,
    children: AtomicU32,
    pub(super) ledger: Mutex<Option<PathBuf>>,
}

impl Board {
    fn mark(&self, mark: &'static str) {
        self.marks.lock().unwrap().insert(mark);
        self.moved.notify_all();
    }

    fn wait(&self, mark: &'static str) {
        let marks = self.marks.lock().unwrap();
        let (marks, waited) = self
            .moved
            .wait_timeout_while(marks, SIBLINGS, |marks| !marks.contains(mark))
            .unwrap();
        drop(marks);
        assert!(!waited.timed_out(), "nobody marked {mark}");
    }

    /// The branch the one request in the history was opened on.
    fn branch_opened(&self) -> String {
        let ledger = self.ledger.lock().unwrap().clone().unwrap();
        read(&ledger)
            .iter()
            .find(|(record, _)| record.kind() == EventKind::PrOpened)
            .and_then(|(record, _)| {
                record
                    .data()
                    .as_map()
                    .get("branch")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .expect("the maker opened a request before the checker was sent")
    }

    /// Waits until `count` runs in `lab/kiln` have frozen.
    fn kiln_frozen(&self, count: usize) {
        let ledger = self.ledger.lock().unwrap().clone().unwrap();
        for _ in 0..500 {
            let lines = read(&ledger);
            if lines
                .iter()
                .filter(|(record, _)| {
                    record.kind() == EventKind::RunFrozen
                        && room_of(&lines, record.run()) == Some("lab/kiln")
                })
                .count()
                >= count
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let seen: Vec<String> = read(&ledger)
            .iter()
            .map(|(record, raw)| {
                format!(
                    "{:?}@{} {}",
                    record.kind(),
                    record.addr().map_or("-", Address::as_str),
                    raw.get(raw.len().saturating_sub(160)..).unwrap_or_default()
                )
            })
            .collect();
        panic!("{count} kiln runs never froze: {seen:#?}");
    }
}

pub(super) struct Planners(pub(super) Arc<Board>);

impl accounting::ModelFactory for Planners {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn Model + std::marker::Send>, AxError> {
        Ok(Box::new(Planner {
            part: None,
            turn: 0,
            child: 0,
            board: Arc::clone(&self.0),
        }))
    }
}

/// Which part a run plays in the plan scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    Potter,
    Glazer,
    Lead,
    Child,
    Maker,
    Checker,
}

pub(super) struct Planner {
    part: Option<Part>,
    turn: u32,
    child: u32,
    board: Arc<Board>,
}

impl Model for Planner {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        if req.chat.tools.is_empty() {
            return says("done");
        }
        let part = *self.part.get_or_insert_with(|| part_of(req));
        self.turn = self.turn.saturating_add(1);
        match (part, self.turn) {
            (Part::Potter, 1) => {
                self.board.mark("potter here");
                self.board.wait("glazer here");
                claim_node_one()
            }
            (Part::Potter, 2) => {
                self.board.mark("potter claimed");
                self.board.wait("glazer tried");
                says("held")
            }
            (Part::Glazer, 1) => {
                self.board.mark("glazer here");
                self.board.wait("potter claimed");
                claim_node_one()
            }
            (Part::Glazer, 2) => {
                self.board.mark("glazer tried");
                says("taken by someone else")
            }
            (Part::Lead, 1) => delegate_kiln(),
            (Part::Lead, 2) => {
                self.board.kiln_frozen(1);
                delegate_kiln()
            }
            (Part::Lead, 3) => {
                self.board.kiln_frozen(2);
                says("the kiln is fired")
            }
            (Part::Child, 1) => {
                self.child = self.board.children.fetch_add(1, Ordering::SeqCst);
                claim_node_one()
            }
            (Part::Child, 2) if self.child == 0 => Err(AxError::failure(
                kernel::AxCode::InvalidArgs,
                "fire the kiln",
                "the kiln cracked",
            )
            .with_recovery("delegate the room again")),
            (Part::Maker, 1) => claim_node_one(),
            (Part::Maker, 2) => calls("read", json!({ "file_path": format!("lab/kiln/{NOTES}") })),
            (Part::Maker, 3) => calls(
                "edit",
                json!({
                    "file_path": format!("lab/kiln/{NOTES}"),
                    "old_string": "before",
                    "new_string": "after",
                }),
            ),
            (Part::Maker, 4) => calls("pr", json!({ "action": "open" })),
            (Part::Checker, 1) => calls(
                "pr",
                json!({
                    "action": "check",
                    "branch": self.board.branch_opened(),
                    "passed": false,
                    "why": "the kiln cracked on the second firing",
                }),
            ),
            (
                Part::Potter
                | Part::Glazer
                | Part::Lead
                | Part::Child
                | Part::Maker
                | Part::Checker,
                _,
            ) => says("fired"),
        }
    }
}

fn part_of(req: &ModelRequest) -> Part {
    let text = first_text(req);
    if text.contains(MAKER) {
        Part::Maker
    } else if text.contains(CHECKER) {
        Part::Checker
    } else if text.contains(POTTER) {
        Part::Potter
    } else if text.contains(GLAZER) {
        Part::Glazer
    } else if text.contains(CHILD_GOAL) {
        Part::Child
    } else {
        Part::Lead
    }
}

fn first_text(req: &ModelRequest) -> String {
    req.chat
        .messages
        .first()
        .into_iter()
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            ContentBlock::Text { text, .. } => Some(text.as_str()),
            ContentBlock::ToolResult { .. }
            | ContentBlock::Thinking { .. }
            | ContentBlock::RedactedThinking { .. }
            | ContentBlock::ToolUse { .. }
            | ContentBlock::Image(_) => None,
        })
        .chain(req.chat.system.iter().map(|block| block.text.as_str()))
        .collect()
}

fn claim_node_one() -> Result<ModelReturn, AxError> {
    calls("plan", json!({ "action": "claim", "node": "1" }))
}

fn delegate_kiln() -> Result<ModelReturn, AxError> {
    calls(
        "delegate",
        json!({ "room": "lab/kiln", "task": CHILD_TASK, "goal": CHILD_GOAL }),
    )
}

/// Writes the one-node plan into `lab`, where the plan tool reads it.
pub(super) fn plan_lab(dir: &Path) {
    std::fs::write(
        CityLayout::new(dir)
            .scope(&Address::parse("lab").unwrap())
            .join(kernel::ROADMAP_FILE),
        PLAN,
    )
    .unwrap();
}

/// The plan's lines, the runs' starts and freezes, and the answers to
/// `plan` calls, in ledger order, for the lines `keep` admits: kind,
/// room, and the one word that decides the scenario.
pub(super) fn plan_trace(
    lines: &Lines,
    keep: impl Fn(&(EventRecord, String)) -> bool,
) -> Vec<String> {
    lines
        .iter()
        .filter(|line| keep(line))
        .filter_map(|(record, _)| {
            let room = record
                .addr()
                .map(Address::as_str)
                .or_else(|| room_of(lines, record.run()))
                .unwrap_or("-");
            let kind = record.kind();
            let word = if kind == EventKind::ToolResult {
                // A run's own failure is written on a `tool_result` line
                // whose payload is the error alone, with no call it
                // answers.
                match record.data().read::<kernel::event::record::ToolResult>() {
                    Ok(data) if data.name.as_str() != "plan" => return None,
                    Ok(data) => match data.answer {
                        kernel::event::record::ToolAnswer::Answered { .. } => " plan answered",
                        kernel::event::record::ToolAnswer::Failed { .. } => " plan failed",
                    }
                    .to_owned(),
                    Err(_) => " run failed".to_owned(),
                }
            } else if kind == EventKind::RunFrozen {
                let data: kernel::event::record::RunFrozen = record.data().read().unwrap();
                format!(" {}", data.completion)
            } else if PLAN_KINDS.contains(&kind) {
                String::new()
            } else {
                return None;
            };
            Some(format!("{:?} {room}{word}", record.kind()))
        })
        .collect()
}

/// The lines a plan trace names with no word beside the kind.
const PLAN_KINDS: [EventKind; 5] = [
    EventKind::RoadmapClaimed,
    EventKind::RoadmapReleased,
    EventKind::RoadmapFinished,
    EventKind::RoadmapBlocked,
    EventKind::RunStarted,
];

/// The room a run was started in.
pub(super) fn room_of(lines: &Lines, run: RunId) -> Option<&str> {
    lines
        .iter()
        .find(|(record, _)| record.kind() == EventKind::RunStarted && record.run() == run)
        .and_then(|(record, _)| record.addr().map(Address::as_str))
}
