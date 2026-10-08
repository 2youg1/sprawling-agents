// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The records of the chosen room, as the CLI prints them
//! (`crates/sprawling/spec/Console.lean` §8-11, sprawling D44).
//!
//! The CLI prints a model's reply whole and a tool call as one line,
//! because the person chose the CLI, which is the condition under which
//! a payload is shown at all. Records of other rooms and of the city
//! print nothing: the terminal is not an event stream.
//!
//! Pure: [`Room::read`] takes one record and answers the lines to print,
//! keeping what the CLI needs to act on (the run working here, the
//! requests waiting) as it goes.

use kernel::{Address, ApprovalId, EventKind, EventRecord, RunId};

/// What the CLI knows about its room from the records it has read.
#[derive(Debug, Default)]
pub(crate) struct Room {
    /// The run working in the room, which `/stop` and Esc cancel.
    pub(crate) run: Option<RunId>,
    /// The requests waiting for an answer, oldest first.
    pub(crate) waiting: Vec<ApprovalId>,
}

impl Room {
    /// Forgets what was read about the room the CLI just left.
    pub(crate) fn clear(&mut self) {
        self.run = None;
        self.waiting.clear();
    }

    /// The lines one record prints, when it belongs to `room`.
    pub(crate) fn read(&mut self, record: &EventRecord, room: &Address) -> Vec<String> {
        if record.addr() != Some(room) {
            return Vec::new();
        }
        let data = record.data().as_map();
        let text = |key: &str| data.get(key).and_then(serde_json::Value::as_str);
        let kind = record.kind();
        if kind == EventKind::RunStarted {
            self.run = Some(record.run());
            Vec::new()
        } else if kind == EventKind::RunFrozen {
            if self.run == Some(record.run()) {
                self.run = None;
            }
            Vec::new()
        } else if kind == EventKind::ModelReturned {
            replied(data.get("message"))
        } else if kind == EventKind::ToolCalled {
            vec![format!(
                "    {:<8} {}",
                text("name").unwrap_or("tool"),
                text("subject").unwrap_or("")
            )]
        } else if kind == EventKind::ApprovalRequested {
            self.requested(data)
        } else if kind == EventKind::ApprovalResolved {
            if let Some(id) = text("id") {
                self.waiting.retain(|waiting| waiting.as_str() != id);
            }
            Vec::new()
        } else {
            Vec::new()
        }
    }
}

impl Room {
    /// A request that waits for the person: one line with the two keys
    /// that answer it, and its identity kept for `y`, `n` and `/approve`.
    fn requested(&mut self, data: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
        match serde_json::from_value::<kernel::ApprovalItem>(serde_json::Value::Object(
            data.clone(),
        )) {
            Ok(item) => {
                let line = format!("  ? {}      y approve   n deny", item.action_desc);
                self.waiting.push(item.id);
                vec![line]
            }
            Err(err) => vec![format!(
                "  ? a request is waiting that this terminal cannot read ({err}); answer it in the WebUI"
            )],
        }
    }
}

/// The text blocks of a model's reply, each line indented under the
/// first so a reply reads as one paragraph beside the prompt.
fn replied(message: Option<&serde_json::Value>) -> Vec<String> {
    let blocks = message
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_array);
    blocks
        .into_iter()
        .flatten()
        .filter(|block| block.get("type").and_then(serde_json::Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text").and_then(serde_json::Value::as_str))
        .flat_map(str::lines)
        .map(|line| format!("  {line}"))
        .collect()
}
