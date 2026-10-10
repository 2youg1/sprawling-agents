// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The records of the chosen room, as the CLI shows them
//! (`crates/sprawling/spec/Console.lean` §8-11, sprawling D44).
//!
//! The CLI shows a run of the chosen room as the WebUI's conversation
//! does: a head when the run first asks its model, the length of what the
//! model reasoned, each tool call once it has answered with how long it
//! took, the reply whole, and how the run ended. Records of other rooms
//! and of the city show nothing: the terminal is not an event stream.
//!
//! A run's own lines - its requests, calls, replies and freeze - carry
//! no address; they are the room's because its resident wrote them, or
//! because they belong to the run working there.
//!
//! Pure: [`Room::read`] takes one record and answers the transcript
//! lines it adds, keeping what the live region draws as it goes - the
//! run working here and since when, the calls under way, and the
//! requests waiting - and what the CLI acts on: the run `/stop` and Esc
//! cancel, and the request `y` and `n` answer.

use console_ffi::part::{Ending, Outcome, Verdict};
use console_ffi::scene::Entry;
use kernel::event::record::{ApprovalResolved, ModelCalled, RunFrozen, RunStarted, ToolCalled};
use kernel::event::record::{ToolAnswer, ToolResult};
use kernel::{Address, ApprovalId, Completion, EventKind, EventRecord, RunId, TimeMs};

use super::local_time::Zone;

/// What the CLI knows about its room from the records it has read.
#[derive(Debug, Default)]
pub(crate) struct Room {
    /// The run working in the room, which `/stop` and Esc cancel.
    pub(crate) run: Option<RunId>,
    /// The requests waiting for an answer, oldest first.
    pub(crate) waiting: Vec<Waiting>,
    /// The tool calls under way, oldest first.
    pub(crate) calls: Vec<Call>,
    /// When the working run started, and what its opening froze.
    opening: Option<Opening>,
}

/// A request waiting for the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Waiting {
    pub(crate) id: ApprovalId,
    pub(crate) what: String,
}

/// A tool call that has not answered yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    id: String,
    pub(crate) name: String,
    pub(crate) subject: String,
    at: TimeMs,
}

/// The working run's start, as its `run_started` recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Opening {
    pub(crate) since: TimeMs,
    mode: Option<kernel::Mode>,
    effort: Option<kernel::Effort>,
    /// Whether the head is already drawn: it is drawn once, at the
    /// run's first request, because only that record names the model.
    headed: bool,
}

impl Room {
    /// Forgets what was read about the room the CLI just left.
    pub(crate) fn clear(&mut self) {
        *self = Room::default();
    }

    /// When the working run started.
    pub(crate) fn since(&self) -> Option<TimeMs> {
        self.opening.as_ref().map(|opening| opening.since)
    }

    /// The transcript lines one record adds, when it belongs to `room`.
    pub(crate) fn read(&mut self, record: &EventRecord, room: &Address, zone: Zone) -> Vec<Entry> {
        if !self.belongs(record, room) {
            return Vec::new();
        }
        let at = zone(record.t());
        let kind = record.kind();
        if kind == EventKind::RunStarted {
            self.started(record);
            Vec::new()
        } else if kind == EventKind::ModelCalled {
            self.head(record, room, zone).into_iter().collect()
        } else if kind == EventKind::ModelReturned {
            replied(record)
        } else if kind == EventKind::ToolCalled {
            self.called(record)
        } else if kind == EventKind::ToolResult {
            self.answered(record, zone)
        } else if kind == EventKind::ApprovalRequested {
            self.requested(record)
        } else if kind == EventKind::ApprovalResolved {
            self.resolved(record, at)
        } else if kind == EventKind::RunFrozen {
            self.frozen(record, at)
        } else {
            Vec::new()
        }
    }

    /// Whether a record is the room's: addressed to it, as `run_started`
    /// is; written by its resident, as a run's calls and replies are,
    /// which carry no address; or a line of the run working there.
    fn belongs(&self, record: &EventRecord, room: &Address) -> bool {
        record.addr() == Some(room)
            || record.who() == room.as_str()
            || self.run == Some(record.run())
    }

    fn started(&mut self, record: &EventRecord) {
        let started = record.data().read::<RunStarted>().ok();
        self.run = Some(record.run());
        self.calls.clear();
        self.opening = Some(Opening {
            since: record.t(),
            mode: started
                .as_ref()
                .and_then(|it| it.policy.as_ref())
                .map(|policy| policy.mode),
            effort: started.and_then(|it| it.effort),
            headed: false,
        });
    }

    /// The run's head, at its first request: who answers, the model,
    /// the mode and the thinking level its opening froze.
    fn head(&mut self, record: &EventRecord, room: &Address, zone: Zone) -> Option<Entry> {
        let opening = self.opening.as_mut().filter(|opening| !opening.headed)?;
        opening.headed = true;
        let model = record
            .data()
            .read::<ModelCalled>()
            .map(|called| called.model)
            .unwrap_or_default();
        let facts = [
            Some(model).filter(|model| !model.is_empty()),
            opening.mode.map(|mode| mode.as_str().to_owned()),
            opening.effort.map(|effort| effort.as_str().to_owned()),
        ];
        Some(Entry::Head {
            at: zone(record.t()),
            resident: resident(room).to_owned(),
            facts: facts.into_iter().flatten().collect(),
        })
    }

    fn called(&mut self, record: &EventRecord) -> Vec<Entry> {
        match record.data().read::<ToolCalled>() {
            Ok(called) => self.calls.push(Call {
                id: called.id,
                name: called.name.as_str().to_owned(),
                subject: called.subject.unwrap_or_default(),
                at: record.t(),
            }),
            Err(err) => return vec![unreadable("a tool call", &err)],
        }
        Vec::new()
    }

    /// A call that answered: one line with its duration, timed from the
    /// moment it was called. A result whose call this console never saw
    /// is drawn at its own moment, without a subject.
    fn answered(&mut self, record: &EventRecord, zone: Zone) -> Vec<Entry> {
        let result = match record.data().read::<ToolResult>() {
            Ok(result) => result,
            Err(err) => return vec![unreadable("a tool result", &err)],
        };
        let call = self
            .calls
            .iter()
            .position(|call| call.id == result.tool_use_id)
            .map(|index| self.calls.remove(index));
        let called_at = call.as_ref().map_or(record.t(), |call| call.at);
        let took_us = result.took_us.or_else(|| {
            record
                .t()
                .value()
                .checked_sub(called_at.value())
                .and_then(|millis| millis.checked_mul(1_000))
        });
        let outcome = match result.answer {
            ToolAnswer::Answered { .. } => Outcome::Answered,
            ToolAnswer::Failed { .. } => Outcome::Failed,
        };
        let (name, subject) = call.map_or_else(
            || (result.name.as_str().to_owned(), String::new()),
            |call| (call.name, call.subject),
        );
        vec![Entry::Tool {
            at: zone(called_at),
            name,
            subject,
            took_us,
            outcome,
        }]
    }

    /// A request that waits for the person: kept for the live region,
    /// for `y`, `n` and `/approve`; it reaches the transcript once it is
    /// answered.
    fn requested(&mut self, record: &EventRecord) -> Vec<Entry> {
        match record.data().read::<kernel::ApprovalItem>() {
            Ok(item) => {
                self.waiting.push(Waiting {
                    id: item.id,
                    what: item.action_desc,
                });
                Vec::new()
            }
            Err(err) => vec![Entry::Note {
                said: format!(
                    "a request is waiting that this terminal cannot read ({err}); answer it in the WebUI"
                ),
            }],
        }
    }

    fn resolved(
        &mut self,
        record: &EventRecord,
        at: Option<console_ffi::scene::TimeOfDay>,
    ) -> Vec<Entry> {
        let Ok(answered) = record.data().read::<ApprovalResolved>() else {
            return Vec::new();
        };
        let Some(index) = self
            .waiting
            .iter()
            .position(|waiting| waiting.id == answered.id)
        else {
            return Vec::new();
        };
        let waiting = self.waiting.remove(index);
        let verdict = match answered.verdict {
            kernel::Ruling::Allow => Verdict::Approved,
            kernel::Ruling::Deny => Verdict::Denied,
        };
        vec![Entry::Resolved {
            at,
            verdict,
            what: waiting.what,
        }]
    }

    fn frozen(
        &mut self,
        record: &EventRecord,
        at: Option<console_ffi::scene::TimeOfDay>,
    ) -> Vec<Entry> {
        let completion = record
            .data()
            .read::<RunFrozen>()
            .map(|frozen| frozen.completion)
            .unwrap_or_default();
        let ending = if completion == Completion::Cancelled.name() {
            Ending::Cancelled
        } else if completion == Completion::Limit.name() {
            Ending::Limit
        } else {
            Ending::Done
        };
        let took_s = self.since().and_then(|since| {
            record
                .t()
                .value()
                .checked_sub(since.value())
                .and_then(|millis| millis.checked_div(1_000))
        });
        if self.run == Some(record.run()) {
            self.run = None;
            self.opening = None;
            self.calls.clear();
        }
        vec![Entry::Ended { at, ending, took_s }]
    }
}

/// Who answers in `room`: the last part of its address.
pub(crate) fn resident(room: &Address) -> &str {
    room.as_str()
        .rsplit('/')
        .next()
        .unwrap_or_else(|| room.as_str())
}

/// The reasoning's length and the reply, out of a `model_returned`.
fn replied(record: &EventRecord) -> Vec<Entry> {
    let data = record.data().as_map();
    let Some(message) = data.get("message") else {
        return Vec::new();
    };
    let reasoning = wire::thought_in(message)
        .map(|thought| thought.chars().count())
        .filter(|characters| *characters > 0)
        .map(|characters| Entry::Reasoning {
            characters: u64::try_from(characters).unwrap_or(u64::MAX),
        });
    let reply = wire::said_in(message)
        .filter(|said| !said.trim().is_empty())
        .map(|said| Entry::Reply { said });
    reasoning.into_iter().chain(reply).collect()
}

fn unreadable(what: &str, err: &kernel::AxError) -> Entry {
    Entry::Note {
        said: format!("{what} could not be read here ({err}); the WebUI shows it"),
    }
}
