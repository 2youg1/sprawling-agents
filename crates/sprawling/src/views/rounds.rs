// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One session's records folded into the rounds a page reads.
//!
//! **This fold used to run in the browser.** `web::turn` held it, so a
//! second client could not draw a session without writing the same fold
//! again, and the wire stopped being the whole API. Card-6.5 moved it
//! here whole: the arms below are the arms that were there, so a session
//! reads the same after the move as before it.
//!
//! Reading one payload is [`channels::reading`]'s, because the client
//! still folds the pushed stream forward into what it believes and that
//! reading has to have one authority.

#[cfg(test)]
mod reading_tests;
#[cfg(test)]
mod tests;

use channels::{EventKind, EventRecord, RunId, UsdMicros};

use super::holding::Views;

impl Views {
    /// The newest [`channels::HISTORY_MAX`] records of one run, oldest
    /// first.
    ///
    /// The same width the client used to ask for with
    /// `Query::RunHistory`, because moving a fold to the server must not
    /// quietly change how much of a session it can see. A line that will
    /// not read ends the slice rather than emptying it - what was read is
    /// still true.
    pub(super) fn records_of(&mut self, run: RunId) -> Vec<EventRecord> {
        let dir = crate::assembly::ledger_dir(&self.city_root);
        if self.index.refresh(&dir).is_err() {
            return Vec::new();
        }
        let want = usize::try_from(channels::HISTORY_MAX).unwrap_or(1);
        let mut newest: Vec<kernel::Seq> =
            self.index.run_seqs_before(run, None).take(want).collect();
        newest.reverse();
        let mut records = Vec::with_capacity(newest.len());
        let mut reader = self.index.reader(&dir);
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
    pub(super) fn rounds_answer(&mut self, run: RunId) -> channels::RoundsAnswer {
        let records = self.records_of(run);
        let turns = turns(records.iter());
        channels::RoundsAnswer {
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
/// The first fence of the session, because that is the tree as the work
/// found it; measuring from the latest one would answer "what moved in
/// the last wave", which is a different question and not the one a
/// person opening a session is asking.
#[must_use]
fn opened_at(turns: &[channels::Turn]) -> Option<kernel::GitOid> {
    turns
        .iter()
        .flat_map(|turn| turn.notes.iter())
        .find_map(|note| match *note {
            channels::Note::Fenced { oid, .. } => Some(oid),
            _ => None,
        })
}

/// How the session opened, from the first `run_started` in the window.
#[must_use]
fn opening(records: &[EventRecord]) -> Option<channels::Opening> {
    records
        .iter()
        .find(|record| record.kind() == EventKind::RunStarted)
        .map(|record| {
            let map = record.data().as_map();
            channels::Opening {
                task: channels::text(map.get("task")).unwrap_or_default(),
                goal: channels::text(map.get("goal")).unwrap_or_default(),
                at: record.t(),
            }
        })
}

/// How the session closed, from the first `run_frozen` in the window.
#[must_use]
fn closing(records: &[EventRecord]) -> Option<channels::Closing> {
    records
        .iter()
        .find(|record| record.kind() == EventKind::RunFrozen)
        .map(|record| channels::Closing {
            completion: channels::text(record.data().as_map().get("completion"))
                .unwrap_or_default(),
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
pub(crate) fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<channels::Turn> {
    let mut folded: Vec<channels::Turn> = Vec::new();
    // Which turn each outstanding call sits in, by the id the runtime
    // gave it. Answers arrive after other calls have been made, so the
    // pairing cannot be positional.
    let mut awaiting: Vec<(String, usize, usize)> = Vec::new();
    for record in records {
        match record.kind() {
            EventKind::ModelCalled => {
                let number = u32::try_from(folded.len().saturating_add(1)).unwrap_or(u32::MAX);
                folded.push(channels::Turn {
                    number,
                    opened: record.seq(),
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
                let call = channels::Call {
                    tool: channels::text(map.get("name")).unwrap_or_else(|| "tool".to_owned()),
                    subject: channels::subject_of(map.get("args")),
                    outcome: channels::Outcome::Waiting,
                    at: record.seq(),
                    output: None,
                };
                let Some(turn) = folded.get_mut(turn_at) else {
                    continue;
                };
                if let Some(id) = channels::text(map.get("id")) {
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
                turn.said = map.get("message").and_then(channels::said_in);
                turn.thought = map.get("message").and_then(channels::thought_in);
                turn.spent = map
                    .get("billed_usd_micros")
                    .and_then(serde_json::Value::as_u64)
                    .map(UsdMicros::new);
                turn.used = map.get("usage").and_then(channels::used_in);
                turn.stopped = channels::text(map.get("stop"));
            }
            EventKind::ToolResult => {
                let map = record.data().as_map();
                let Some(id) = channels::text(map.get("tool_use_id")) else {
                    continue;
                };
                let Some(at) = awaiting.iter().position(|(held, _, _)| held == &id) else {
                    continue;
                };
                let (_, turn_at, call_at) = awaiting.swap_remove(at);
                let (outcome, said) = match map.get("error") {
                    Some(failed) => (channels::Outcome::Failed, Some(failed)),
                    None => (channels::Outcome::Answered, map.get("result")),
                };
                if let Some(call) = folded
                    .get_mut(turn_at)
                    .and_then(|turn| turn.calls.get_mut(call_at))
                {
                    call.outcome = outcome;
                    call.output = said.and_then(channels::output_in);
                }
            }
            kind => {
                // Everything else is a note when it changed what this
                // turn did or what it waits on, and nothing otherwise.
                let Some(note) = channels::note_of(kind, record) else {
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
