// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{
    ApprovalResolved, Landing, SignalConsumed, SignalKind, SignalLanded, SignalWaitEnded, WaitEnd,
};
use wire::{EventKind, EventRecord};

use crate::views::prepared::LedgerAsk;

const SENDING_REACH: usize = 4096;

struct Sending {
    from: String,
    said: Option<String>,
    kind: SignalKind,

    session: kernel::RunId,
    handback: Option<wire::HandbackNote>,
}

impl LedgerAsk {
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

pub(super) fn land_sends(turns: &mut [wire::Turn], records: &[EventRecord]) {
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
