// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Rounds a person reads: turns folded from a session’s events.

use channels::{EventKind, EventRecord, GitOid, UsdMicros};

use super::reading::Turn;
use super::reading::{Call, Note, Outcome, note_of, output_in, said_in, subject_of, text, used_in};

/// The checkpoint a session's changes are measured from.
///
/// The first fence of the session, because that is the tree as the work
/// found it; measuring from the latest one would answer "what moved in
/// the last wave", which is a different question and not the one a
/// person opening a session is asking.
#[must_use]
pub fn opened_at(turns: &[Turn]) -> Option<GitOid> {
    turns
        .iter()
        .flat_map(|turn| turn.notes.iter())
        .find_map(|note| match *note {
            Note::Fenced { oid, .. } => Some(oid),
            _ => None,
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
pub fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<Turn> {
    let mut folded: Vec<Turn> = Vec::new();
    // Which turn each outstanding call sits in, by the id the runtime
    // gave it. Answers arrive after other calls have been made, so the
    // pairing cannot be positional.
    let mut awaiting: Vec<(String, usize, usize)> = Vec::new();
    for record in records {
        match record.kind() {
            EventKind::ModelCalled => {
                let number = u32::try_from(folded.len().saturating_add(1)).unwrap_or(u32::MAX);
                folded.push(Turn {
                    number,
                    opened: record.seq(),
                    said: None,
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
                let call = Call {
                    tool: text(map.get("name")).unwrap_or_else(|| "tool".to_owned()),
                    subject: subject_of(map.get("args")),
                    outcome: Outcome::Waiting,
                    at: record.seq(),
                    output: None,
                };
                let Some(turn) = folded.get_mut(turn_at) else {
                    continue;
                };
                if let Some(id) = text(map.get("id")) {
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
                turn.said = map.get("message").and_then(said_in);
                turn.spent = map
                    .get("billed_usd_micros")
                    .and_then(serde_json::Value::as_u64)
                    .map(UsdMicros::new);
                turn.used = map.get("usage").and_then(used_in);
                turn.stopped = text(map.get("stop"));
            }
            EventKind::ToolResult => {
                let map = record.data().as_map();
                let Some(id) = text(map.get("tool_use_id")) else {
                    continue;
                };
                let Some(at) = awaiting.iter().position(|(held, _, _)| held == &id) else {
                    continue;
                };
                let (_, turn_at, call_at) = awaiting.swap_remove(at);
                let (outcome, said) = match map.get("error") {
                    Some(failed) => (Outcome::Failed, Some(failed)),
                    None => (Outcome::Answered, map.get("result")),
                };
                if let Some(call) = folded
                    .get_mut(turn_at)
                    .and_then(|turn| turn.calls.get_mut(call_at))
                {
                    call.outcome = outcome;
                    call.output = said.and_then(output_in);
                }
            }
            kind => {
                // Everything else is a note when it changed what this
                // turn did or what it waits on, and nothing otherwise.
                let Some(note) = note_of(kind, record) else {
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
