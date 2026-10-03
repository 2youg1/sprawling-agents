// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The notes whose other half sits on another line
//! (`crates/accounting/spec/Views/Rounds.lean` D48): an answer to a
//! wait, a pulled signal and its sending, a reply wait and its end.
//!
//! `turns` stays a pure fold over one session's records; these passes
//! run after it, and the one that reads other runs goes through the
//! Ledger. A half that cannot be found leaves its field `None` rather
//! than a guessed value.

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{ApprovalResolved, SignalConsumed, SignalWaitEnded, WaitEnd};
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
    handback: Option<wire::HandbackNote>,
}

impl LedgerAsk {
    /// Writes onto each arrival in `turns` who sent it, what it said, and
    /// whether it handed work back, from its `signal_enqueued` line
    /// under the sender's run (wire D36, D38).
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
                ..
            } = note
                && let Some(sending) = taken.get(at).and_then(|id| sent.get(id))
            {
                *from = Some(sending.from.clone());
                said.clone_from(&sending.said);
                handback.clone_from(&sending.handback);
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
        let Some((index, dir)) = self.indexed() else {
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
            session: record.run(),
        }),
        Ok(Some(collab::Handback::Stopped { because, .. })) => Some(wire::HandbackNote::Stopped {
            because,
            session: record.run(),
        }),
        Ok(None) | Err(_) => None,
    };
    Some((
        signal.id().as_str().to_owned(),
        Sending {
            from: signal.from().to_owned(),
            said: wire::text(signal.payload().as_map().get("text")),
            handback,
        },
    ))
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
