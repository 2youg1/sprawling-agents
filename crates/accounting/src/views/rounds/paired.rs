// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The notes and calls whose other half sits on another line
//! (`crates/accounting/spec/Views/Rounds.lean` D48): an answer to a
//! wait, a pulled signal and its sending, a reply wait and its end, a
//! `send` call and where its letter landed.
//!
//! `turns` stays a pure fold over one session's records; these passes
//! run after it, and the one that reads other runs goes through the
//! Ledger. A half that cannot be found leaves its field `None` rather
//! than a guessed value.

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{
    ApprovalResolved, Landing, SignalConsumed, SignalKind, SignalLanded, SignalWaitEnded, WaitEnd,
};
use wire::{EventKind, EventRecord};

use crate::views::prepared::LedgerAsk;

/// How many Ledger lines before the newest taking the search for a
/// sending reads, across every run. A sending further back is reported
/// as unknown (wire D36).
const SENDING_REACH: usize = 4096;

/// What a pulled signal's sending line says.
struct Sending {
    from: String,
    said: Option<String>,
    kind: SignalKind,
    /// The run the sending line was written under (wire D43).
    session: kernel::RunId,
    handback: Option<wire::HandbackNote>,
}

impl LedgerAsk {
    /// Writes onto each arrival in `turns` who sent it, what it said, and
    /// whether it handed work back, from its `signal_enqueued` line
    /// under the sender's run, with the kind it was sent as and that run
    /// (wire D36, D38, D43).
    pub(super) fn pair_arrivals(&self, turns: &mut [wire::Turn], records: &[EventRecord]) {
        let taken: BTreeMap<kernel::Seq, String> = records
            .iter()
            .filter(|record| record.kind() == EventKind::SignalConsumed)
            .filter_map(|record| {
                let taken = record.data().read::<SignalConsumed>().ok()?;
                Some((record.seq(), taken.id.as_str().to_owned()))
            })
            .collect();
        let Some(newest) = taken.keys().next_back().copied() else {
            return;
        };
        let sent = self.sendings(taken.values().map(String::as_str).collect(), newest);
        for note in turns.iter_mut().flat_map(|turn| turn.notes.iter_mut()) {
            if let wire::Note::Arrived {
                at,
                from,
                said,
                handback,
                kind,
                session,
                ..
            } = note
                && let Some(sending) = taken.get(at).and_then(|id| sent.get(id))
            {
                *from = Some(sending.from.clone());
                said.clone_from(&sending.said);
                handback.clone_from(&sending.handback);
                *kind = Some(sending.kind);
                *session = Some(sending.session);
            }
        }
    }

    /// The sending of each id in `wanted`, searched backward from
    /// `newest` across every run, at most [`SENDING_REACH`] lines. A
    /// line that does not read is passed over: what it would have said
    /// stays unknown, and the other sendings are still true.
    fn sendings(
        &self,
        mut wanted: BTreeSet<&str>,
        newest: kernel::Seq,
    ) -> BTreeMap<String, Sending> {
        let mut found = BTreeMap::new();
        let Ok((index, dir)) = self.indexed() else {
            return found;
        };
        let mut reader = index.reader(&dir);
        let before = index
            .seqs()
            .rev()
            .skip_while(|seq| *seq >= newest)
            .take(SENDING_REACH);
        for seq in before {
            if wanted.is_empty() {
                break;
            }
            let Ok(line) = reader.line_at(seq) else {
                continue;
            };
            // Only a line naming a wanted id is parsed.
            let names = |id: &&str| line.windows(id.len()).any(|held| held == id.as_bytes());
            if !wanted.iter().any(names) {
                continue;
            }
            let Some((id, sending)) = EventRecord::parse_line(&line)
                .ok()
                .filter(|record| record.kind() == EventKind::SignalEnqueued)
                .and_then(|record| sending_of(&record))
            else {
                continue;
            };
            if wanted.remove(id.as_str()) {
                found.insert(id, sending);
            }
        }
        found
    }
}

/// One `signal_enqueued` line read through its writer's inverse, with
/// the id it was sent under.
fn sending_of(record: &EventRecord) -> Option<(String, Sending)> {
    let signal = collab::Signal::from_payload(record.data()).ok()?;
    let handback = match collab::Handback::from_signal(&signal) {
        Ok(Some(collab::Handback::Finished(artifact))) => Some(wire::HandbackNote::Finished {
            verified_by: artifact.verified_by().to_owned(),
        }),
        Ok(Some(collab::Handback::Stopped { because, .. })) => {
            Some(wire::HandbackNote::Stopped { because })
        }
        Ok(None) | Err(_) => None,
    };
    Some((
        signal.id().as_str().to_owned(),
        Sending {
            from: signal.from().to_owned(),
            said: wire::text(signal.payload().as_map().get("text")),
            kind: signal.kind(),
            session: record.run(),
            handback,
        },
    ))
}

/// Writes onto each `send` call in `turns` where its letter landed
/// (wire D42, kernel D38).
///
/// All three lines sit in the sending session: the call, the
/// `signal_enqueued` its desk wrote before the call answered, and the
/// `signal_landed` every delivery path writes under the sending run. A
/// letter is paired to the earliest call still open that names its
/// room, so two sends in flight at once are told apart by where they
/// went, not by the order their lines arrived. A call left unpaired
/// keeps `None`, which a page draws as the tool's own answer.
pub(super) fn land_sends(turns: &mut [wire::Turn], records: &[EventRecord]) {
    // Calls that named a room and have not answered: their tool-use id,
    // their seq, and the room.
    let mut open: Vec<(String, kernel::Seq, String)> = Vec::new();
    let mut letters: BTreeMap<String, kernel::Seq> = BTreeMap::new();
    let mut landed: BTreeMap<kernel::Seq, Landing> = BTreeMap::new();
    for record in records {
        let map = record.data().as_map();
        if record.kind() == EventKind::ToolCalled {
            if let Some(sent) = sending_call(map) {
                open.push((sent.0, record.seq(), sent.1));
            }
        } else if record.kind() == EventKind::ToolResult {
            if let Some(id) = wire::text(map.get("tool_use_id")) {
                open.retain(|(held, _, _)| held != &id);
            }
        } else if record.kind() == EventKind::SignalEnqueued {
            let Ok(signal) = collab::Signal::from_payload(record.data()) else {
                continue;
            };
            if let Some(at) = open
                .iter()
                .position(|(_, _, room)| room == signal.room().as_str())
            {
                let (_, call, _) = open.remove(at);
                letters.insert(signal.id().as_str().to_owned(), call);
            }
        } else if record.kind() == EventKind::SignalLanded
            && let Ok(line) = record.data().read::<SignalLanded>()
            && let Some(call) = letters.get(line.signal.as_str())
        {
            landed.insert(*call, line.landing);
        }
    }
    for call in turns.iter_mut().flat_map(|turn| turn.calls.iter_mut()) {
        call.landing = landed.get(&call.at).copied();
    }
}

/// The tool-use id and the room of a `tool_called` line that is a
/// `signal` call naming an address to send to; `None` for every other
/// call. The registered render intent says it is the signal tool, so the
/// tool's name is not spelled again here.
fn sending_call(map: &serde_json::Map<String, serde_json::Value>) -> Option<(String, String)> {
    let render = map
        .get("render")
        .and_then(|value| <kernel::RenderIntent as serde::Deserialize>::deserialize(value).ok());
    if render != Some(kernel::RenderIntent::Signal) {
        return None;
    }
    let to = map
        .get("args")
        .and_then(serde_json::Value::as_object)
        .and_then(|args| wire::text(args.get("to")))
        .and_then(|to| kernel::Address::parse(&to).ok())?;
    Some((wire::text(map.get("id"))?, to.as_str().to_owned()))
}

/// Writes onto each reply wait in `turns` how and when it ended, from
/// the `signal_wait_ended` line in the same session that names its
/// signal (wire D37).
pub(super) fn end_reply_waits(turns: &mut [wire::Turn], records: &[EventRecord]) {
    let started: BTreeMap<kernel::Seq, String> = records
        .iter()
        .filter(|record| record.kind() == EventKind::SignalWaitStarted)
        .filter_map(|record| {
            let started = record
                .data()
                .read::<kernel::event::record::SignalWaitStarted>()
                .ok()?;
            Some((record.seq(), started.signal.as_str().to_owned()))
        })
        .collect();
    let ended: BTreeMap<String, wire::ReplyEnded> = records
        .iter()
        .filter(|record| record.kind() == EventKind::SignalWaitEnded)
        .filter_map(|record| {
            let ended = record.data().read::<SignalWaitEnded>().ok()?;
            let by = match ended.by {
                WaitEnd::Reply { .. } => wire::ReplyEnd::Reply,
                WaitEnd::Timeout => wire::ReplyEnd::Timeout,
                WaitEnd::Left => wire::ReplyEnd::Left,
            };
            Some((
                ended.signal.as_str().to_owned(),
                wire::ReplyEnded { by, t: record.t() },
            ))
        })
        .collect();
    for note in turns.iter_mut().flat_map(|turn| turn.notes.iter_mut()) {
        if let wire::Note::AwaitingReply { at, ended: end, .. } = note {
            *end = started
                .get(at)
                .and_then(|signal| ended.get(signal))
                .copied();
        }
    }
}

/// Writes onto each wait in `turns` when its answer was recorded.
///
/// `approval_resolved` is recorded under the city's own run, not under
/// the session that asked, so `asked` (the session's records) gives each
/// request's approval id and `city` gives each answer's time. A request
/// or answer outside its window, or a payload that will not read back,
/// leaves `answered` at `None` rather than a guessed end.
pub(super) fn answer_waits(turns: &mut [wire::Turn], asked: &[EventRecord], city: &[EventRecord]) {
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
