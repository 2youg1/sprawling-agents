// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::part::{Ending, Outcome, Verdict};
use crate::scene::Entry;
use kernel::event::record::{ApprovalResolved, ModelCalled, RunFrozen, RunStarted, ToolCalled};
use kernel::event::record::{ToolAnswer, ToolResult};
use kernel::{Address, ApprovalId, Completion, EventKind, EventRecord, RunId, TimeMs};

use super::local_time::Zone;

#[derive(Debug, Default)]
pub(crate) struct Room {
    pub(crate) run: Option<RunId>,
    pub(crate) waiting: Vec<Waiting>,
    pub(crate) calls: Vec<Call>,
    opening: Option<Opening>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Waiting {
    pub(crate) id: ApprovalId,
    pub(crate) what: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    id: String,
    pub(crate) name: String,
    pub(crate) subject: String,
    at: TimeMs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Opening {
    pub(crate) since: TimeMs,
    mode: Option<kernel::Mode>,
    effort: Option<kernel::Effort>,
    headed: bool,
}

impl Room {
    pub(crate) fn clear(&mut self) {
        *self = Room::default();
    }

    pub(crate) fn since(&self) -> Option<TimeMs> {
        self.opening.as_ref().map(|opening| opening.since)
    }

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
        at: Option<crate::scene::TimeOfDay>,
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

    fn frozen(&mut self, record: &EventRecord, at: Option<crate::scene::TimeOfDay>) -> Vec<Entry> {
        let working = self.run == Some(record.run());
        if !working && self.run.is_some() {
            return Vec::new();
        }
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
        let took_s = self.since().filter(|_| working).and_then(|since| {
            record
                .t()
                .value()
                .checked_sub(since.value())
                .and_then(|millis| millis.checked_div(1_000))
        });
        self.run = None;
        self.opening = None;
        self.calls.clear();
        vec![Entry::Ended { at, ending, took_s }]
    }
}

pub(crate) fn resident(room: &Address) -> &str {
    room.as_str()
        .rsplit('/')
        .next()
        .unwrap_or_else(|| room.as_str())
}

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
