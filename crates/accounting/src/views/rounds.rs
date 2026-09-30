// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session's records folded into the rounds a page reads.
//!
//! **This fold runs on the server, not in the browser.** A fold held by
//! one client would make a second client write the same fold again to
//! draw a session, and the wire would stop being the whole API.
//!
//! Reading one payload is [`wire::reading`]'s, because the client
//! still folds the pushed stream forward into what it believes and that
//! reading has to have one authority.

#[cfg(test)]
mod carried_tests;
#[cfg(test)]
mod reading_tests;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use kernel::event::record::ApprovalResolved;
use wire::{EventKind, EventRecord, RunId, UsdMicros};

use super::prepared::LedgerAsk;

impl LedgerAsk {
    /// The newest [`wire::HISTORY_MAX`] records of one run, oldest
    /// first.
    ///
    /// The same width a client asks for with `Query::RunHistory`, so the
    /// fold sees as much of a session as a client reading the history
    /// would. A line that will
    /// not read ends the slice rather than emptying it - what was read is
    /// still true.
    pub(super) fn records_of(&self, run: RunId) -> Vec<EventRecord> {
        let Some((index, dir)) = self.indexed() else {
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

    /// One session, folded into the rounds a person reads.
    pub(super) fn rounds_answer(&self, run: RunId) -> wire::RoundsAnswer {
        let records = self.records_of(run);
        let mut turns = turns(records.iter());
        if records
            .iter()
            .any(|record| record.kind() == EventKind::ApprovalRequested)
        {
            answer_waits(&mut turns, &records, &self.records_of(RunId::CITY));
        }
        wire::RoundsAnswer {
            opened_at: opened_at(&turns),
            opening: opening(&records),
            closing: closing(&records),
            turns,
            run,
        }
    }
}

/// The checkpoint a session's changes are measured from.
///
/// The first checkpoint of the session, because that is the tree as the work
/// found it; measuring from the latest one would answer "what moved in
/// the last wave", which is a different question and not the one a
/// person opening a session is asking.
#[must_use]
fn opened_at(turns: &[wire::Turn]) -> Option<kernel::GitOid> {
    turns
        .iter()
        .flat_map(|turn| turn.notes.iter())
        .find_map(|note| match note {
            wire::Note::Checkpointed { oid, .. } => Some(*oid),
            // The other four notes say what happened in the session;
            // none of them names the commit it opened at.
            wire::Note::Refused { .. }
            | wire::Note::Waiting { .. }
            | wire::Note::Arrived { .. }
            | wire::Note::Discarded { .. }
            | wire::Note::Unreadable { .. } => None,
        })
}

/// Writes onto each wait in `turns` when its answer was recorded.
///
/// `approval_resolved` is recorded under the city's own run, not under
/// the session that asked, so `asked` (the session's records) gives each
/// request's approval id and `city` gives each answer's time. A request
/// or answer outside its window, or a payload that will not read back,
/// leaves `answered` at `None` rather than a guessed end.
fn answer_waits(turns: &mut [wire::Turn], asked: &[EventRecord], city: &[EventRecord]) {
    let ids: BTreeMap<kernel::Seq, String> = asked
        .iter()
        .filter(|record| record.kind() == EventKind::ApprovalRequested)
        .filter_map(|record| {
            let item = record.data().read::<kernel::ApprovalItem>().ok()?;
            Some((record.seq(), item.id.as_str().to_owned()))
        })
        .collect();
    let answers: BTreeMap<String, kernel::TimeMs> = city
        .iter()
        .filter(|record| record.kind() == EventKind::ApprovalResolved)
        .filter_map(|record| {
            let ruled = record.data().read::<ApprovalResolved>().ok()?;
            Some((ruled.id.as_str().to_owned(), record.t()))
        })
        .collect();
    for note in turns.iter_mut().flat_map(|turn| turn.notes.iter_mut()) {
        if let wire::Note::Waiting { at, answered, .. } = note {
            *answered = ids.get(at).and_then(|id| answers.get(id)).copied();
        }
    }
}

/// How the session opened, from the first `run_started` in the window.
#[must_use]
fn opening(records: &[EventRecord]) -> Option<wire::Opening> {
    records
        .iter()
        .find(|record| record.kind() == EventKind::RunStarted)
        .map(|record| {
            // A `run_started` this build cannot read still opened a
            // session; the page shows it with no words rather than
            // dropping the session from the account.
            let started = record
                .data()
                .read::<kernel::event::record::RunStarted>()
                .unwrap_or_default();
            wire::Opening {
                task: started.task,
                goal: started.goal,
                at: record.t(),
                dispatched_by: started.dispatched_by,
            }
        })
}

/// How the session closed, from the first `run_frozen` in the window.
#[must_use]
fn closing(records: &[EventRecord]) -> Option<wire::Closing> {
    records
        .iter()
        .find(|record| record.kind() == EventKind::RunFrozen)
        .map(|record| wire::Closing {
            completion: wire::text(record.data().as_map().get("completion")).unwrap_or_default(),
            at: record.t(),
        })
}

/// Folds a session's events into turns, oldest first.
///
/// Events before the first `model_called` belong to no turn and are left
/// out: they are the session opening, which the head of the page already
/// states. A `tool_result` with no call in this window is dropped for the
/// same reason - the window is bounded, so its first rows can be answers
/// to calls nobody here saw.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the kinds that open, close and fill a turn are named; every other kind is a note"
)]
pub fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<wire::Turn> {
    let mut folded: Vec<wire::Turn> = Vec::new();
    // Which turn each outstanding call sits in, by the id the runtime
    // gave it. Answers arrive after other calls have been made, so the
    // pairing cannot be positional.
    let mut awaiting: Vec<(String, usize, usize)> = Vec::new();
    for record in records {
        match record.kind() {
            EventKind::ModelCalled => {
                let number = u32::try_from(folded.len().saturating_add(1)).unwrap_or(u32::MAX);
                folded.push(wire::Turn {
                    number,
                    opened: record.seq(),
                    t: record.t(),
                    timing: wire::Timing::Measured,
                    first_at: None,
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
                // Nothing open means this is the session's own opening,
                // which belongs to no round.
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
                    timing: wire::Timing::Measured,
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
                // The answer belongs to the turn the question opened.
                // Nothing open means this record is the session's own
                // opening, which no round owns.
                let Some(turn) = folded.last_mut() else {
                    continue;
                };
                let map = record.data().as_map();
                turn.said = map.get("message").and_then(wire::said_in);
                turn.thought = map.get("message").and_then(wire::thought_in);
                turn.spent = map
                    .get("billed_usd_micros")
                    .and_then(serde_json::Value::as_u64)
                    .map(UsdMicros::new);
                turn.used = map.get("usage").and_then(wire::used_in);
                turn.stopped = wire::text(map.get("stop"));
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
                    call.output = said.and_then(wire::output_in);
                    call.answered = Some(record.t());
                }
            }
            kind => {
                // Everything else is a note when it changed what this
                // turn did or what it waits on, and nothing otherwise.
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
