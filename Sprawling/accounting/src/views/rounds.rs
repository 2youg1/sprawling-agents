// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod opening;
mod paired;

#[cfg(test)]
mod arrival_tests;
#[cfg(test)]
mod carried_tests;
#[cfg(test)]
mod landing_tests;
#[cfg(test)]
mod opening_tests;
#[cfg(test)]
mod reading_tests;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use kernel::Completion;
use wire::{EventKind, EventRecord, RunId, UsdMicros};

use super::prepared::LedgerAsk;

impl LedgerAsk {

    pub(super) fn records_of(&self, run: RunId) -> Vec<EventRecord> {
        let Ok((index, dir)) = self.indexed() else {
            return Vec::new();
        };
        let want = usize::try_from(wire::HISTORY_MAX).unwrap_or(1);
        let mut newest: Vec<kernel::Seq> = index.run_seqs_before(run, None).take(want).collect();
        newest.reverse();
        let mut records = Vec::with_capacity(newest.len());
        let mut reader = index.reader(&dir);
        for seq in newest {
            let Ok(line) = reader.line_at(seq) else {
                break;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                break;
            };
            records.push(record);
        }
        records
    }

    pub(super) fn rounds_answer(&self, run: RunId) -> Result<wire::RoundsAnswer, kernel::AxError> {
        let records = self.records_of(run);
        let mut turns = turns(records.iter());
        if records
            .iter()
            .any(|record| record.kind() == EventKind::ApprovalRequested)
        {
            paired::answer_waits(&mut turns, &records, &self.records_of(RunId::CITY));
        }
        self.pair_arrivals(&mut turns, &records);
        paired::end_reply_waits(&mut turns, &records);
        paired::land_sends(&mut turns, &records);
        Ok(wire::RoundsAnswer {
            opened_at: opened_at(&turns),
            closing: closing(&records)?,
            worktree: worktree(&records),
            opening: opening::opening(&records, &self.city_root),
            turns,
            run,
        })
    }
}

#[must_use]
fn opened_at(turns: &[wire::Turn]) -> Option<kernel::GitOid> {
    turns
        .iter()
        .flat_map(|turn| turn.notes.iter())
        .find_map(|note| match note {
            wire::Note::Checkpointed { oid, .. } => Some(*oid),

            wire::Note::Refused { .. }
            | wire::Note::Waiting { .. }
            | wire::Note::Arrived { .. }
            | wire::Note::AwaitingReply { .. }
            | wire::Note::Discarded { .. }
            | wire::Note::Unreadable { .. } => None,
        })
}

#[must_use]
fn worktree(records: &[EventRecord]) -> Option<String> {
    let opened = records
        .iter()
        .find(|record| record.kind() == EventKind::WorktreeOpened)?;
    match opened
        .data()
        .read::<kernel::event::record::WorktreeOpened>()
    {
        Ok(opened) => Some(opened.name),
        Err(_unreadable) => None,
    }
}

fn closing(records: &[EventRecord]) -> Result<Option<wire::Closing>, kernel::AxError> {
    records
        .iter()
        .find(|record| record.kind() == EventKind::RunFrozen)
        .map(|record| {
            let frozen = record.data().read::<kernel::event::record::RunFrozen>()?;

            let failure = match frozen.failure {
                Some(failed) => Some(failed),
                None if frozen.completion == Completion::Cancelled.name() => {
                    old_carrier_failure(records)
                }
                None => None,
            };
            Ok(wire::Closing {
                completion: frozen.completion,
                at: record.t(),
                failure,
            })
        })
        .transpose()
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "three kinds write a carrier; every other record is not one"
)]
fn old_carrier_failure(records: &[EventRecord]) -> Option<kernel::AxError> {
    records.iter().rev().find_map(|record| match record.kind() {
        EventKind::ProviderDegraded => record
            .data()
            .read::<kernel::event::record::ProviderDegraded>()
            .ok()
            .and_then(|degraded| match degraded {
                kernel::event::record::ProviderDegraded::Refused(error) => Some(error),
                kernel::event::record::ProviderDegraded::VaultFellBack(_) => None,
            }),
        EventKind::BudgetLimit | EventKind::GateDenied => {
            record.data().read::<kernel::AxError>().ok()
        }
        _ => None,
    })
}

#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the kinds that open, close and fill a turn are named; every other kind is a note"
)]
pub fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<wire::Turn> {
    let mut folded: Vec<wire::Turn> = Vec::new();

    let mut awaiting: Vec<(String, usize, usize)> = Vec::new();
    let mut attempts = Attempts::default();
    for record in records {
        attempts.note(record);
        match record.kind() {
            EventKind::ModelCalled => {
                let number = u32::try_from(folded.len().saturating_add(1)).unwrap_or(u32::MAX);
                folded.push(wire::Turn {
                    number,
                    opened: record.seq(),
                    t: record.t(),
                    timing: timing_of(record),
                    first_at: None,
                    returned: None,
                    model: wire::text(record.data().as_map().get("model")),
                    said: None,
                    thought: None,
                    spent: None,
                    used: None,
                    stopped: None,
                    calls: Vec::new(),
                    notes: Vec::new(),
                });
            }
            EventKind::ToolCalled => {

                let turn_at = match folded.len() {
                    0 => continue,
                    open => open.saturating_sub(1),
                };
                let map = record.data().as_map();
                let call = wire::Call {
                    tool: wire::text(map.get("name")).unwrap_or_else(|| "tool".to_owned()),
                    subject: wire::text(map.get("subject")),
                    arguments: map.get("args").and_then(wire::arguments_in),
                    outcome: wire::Outcome::Waiting,
                    at: record.seq(),
                    output: None,
                    called: record.t(),
                    answered: None,
                    timing: timing_of(record),
                    effect: map.get("effect").and_then(|value| {
                        <kernel::Effect as serde::Deserialize>::deserialize(value).ok()
                    }),
                    render: map.get("render").and_then(|value| {
                        <kernel::RenderIntent as serde::Deserialize>::deserialize(value).ok()
                    }),
                    exit_code: None,
                    took_us: None,
                    landing: None,
                };
                let Some(turn) = folded.get_mut(turn_at) else {
                    continue;
                };
                if let Some(id) = wire::text(map.get("id")) {
                    awaiting.push((id, turn_at, turn.calls.len()));
                }
                turn.calls.push(call);
            }
            EventKind::ModelReturned => {

                let Some(turn) = attempts
                    .answered_by(record)
                    .and_then(|opened| folded.iter_mut().rev().find(|turn| turn.opened == opened))
                else {
                    continue;
                };
                let map = record.data().as_map();
                turn.said = map.get("message").and_then(wire::said_in);
                turn.thought = map.get("message").and_then(wire::thought_in);
                turn.spent = map
                    .get("billed_usd_micros")
                    .and_then(serde_json::Value::as_u64)
                    .map(UsdMicros::new);
                let micros = |key: &str| map.get(key).and_then(serde_json::Value::as_u64);
                turn.used = map
                    .get("usage")
                    .and_then(wire::used_in)
                    .map(|used| wire::Used {
                        first_us: micros("first_us"),
                        took_us: micros("took_us"),
                        ..used
                    });
                turn.stopped = wire::text(map.get("stop"));
                turn.first_at = map
                    .get("first_at")
                    .and_then(serde_json::Value::as_u64)
                    .map(kernel::TimeMs::new);
                turn.returned = record.moment();
            }
            EventKind::ToolResult => {
                let map = record.data().as_map();
                let Some(id) = wire::text(map.get("tool_use_id")) else {
                    continue;
                };
                let Some(at) = awaiting.iter().position(|(held, _, _)| held == &id) else {
                    continue;
                };
                let (_, turn_at, call_at) = awaiting.swap_remove(at);
                let (outcome, said) = match map.get("error") {
                    Some(failed) => (wire::Outcome::Failed, Some(failed)),
                    None => (wire::Outcome::Answered, map.get("result")),
                };
                if let Some(call) = folded
                    .get_mut(turn_at)
                    .and_then(|turn| turn.calls.get_mut(call_at))
                {
                    call.outcome = outcome;
                    call.output = said.and_then(wire::output_in).map(|shown| wire::Output {
                        pinned: map.get("result").and_then(runtime::pinned_original),
                        ..shown
                    });
                    call.exit_code = map
                        .get("result")
                        .and_then(serde_json::Value::as_object)
                        .and_then(runtime::exit_code_in);
                    call.answered = Some(record.t());
                    call.took_us = map.get("took_us").and_then(serde_json::Value::as_u64);
                    call.timing = answered_timing(call.timing, record);
                }
            }
            kind => {

                let Some(note) = wire::note_of(kind, record) else {
                    continue;
                };
                if let Some(turn) = folded.last_mut() {
                    turn.notes.push(note);
                }
            }
        }
    }
    folded
}

#[derive(Debug, Default)]
pub(crate) struct Attempts {
    latest: BTreeMap<RunId, kernel::Seq>,
}

impl Attempts {

    pub(crate) fn note(&mut self, record: &EventRecord) {
        if record.kind() == EventKind::ModelCalled {
            self.latest.insert(record.run(), record.seq());
        }
    }

    pub(crate) fn answered_by(&self, reply: &EventRecord) -> Option<kernel::Seq> {
        if reply.kind() == EventKind::ModelReturned {
            self.latest.get(&reply.run()).copied()
        } else {
            None
        }
    }
}

pub(crate) fn timing_of(record: &EventRecord) -> wire::Timing {
    match record.moment() {
        Some(_) => wire::Timing::Measured,
        None => wire::Timing::Unmeasured,
    }
}

pub(crate) fn answered_timing(asked: wire::Timing, answer: &EventRecord) -> wire::Timing {
    if supplied_by_the_city(answer.data().as_map().get("error")) {
        return wire::Timing::Unmeasured;
    }
    match asked {
        wire::Timing::Measured => timing_of(answer),
        wire::Timing::Unmeasured => wire::Timing::Unmeasured,
    }
}

fn supplied_by_the_city(error: Option<&serde_json::Value>) -> bool {
    error
        .and_then(|value| <kernel::AxError as serde::Deserialize>::deserialize(value).ok())
        .is_some_and(|refused| *refused.code() == kernel::AxCode::ToolOutcomeUnknown)
}
