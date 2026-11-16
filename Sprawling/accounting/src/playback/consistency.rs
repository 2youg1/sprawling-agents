// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What makes a bundle consistent on its own (`crates/accounting/spec/Playback.lean` §8-12):
//! ascending seqs, lines that are the ledger lines their entries name,
//! and ends, members and runs that resolve inside it. Consistent says
//! the parts agree; it does not say they were never changed.

use std::collections::BTreeSet;

use kernel::{AxCode, AxError, RunId};
use storage::CheckedLine;

use super::SCHEMA;
use super::document::{CallTrace, Cited, Decimal, Document, End, Event, Holds};

/// Every reference inside a document resolves, and every line is the
/// ledger line its entry says it is.
pub(super) fn consistent(document: &Document) -> Result<(), AxError> {
    if document.schema != SCHEMA {
        return Err(inconsistent(format!(
            "schema '{}', where this build reads '{SCHEMA}'",
            document.schema
        )));
    }
    let events = ascending("events", &document.events)?;
    let context = ascending("context", &document.context)?;
    if let Some(both) = events.intersection(&context).next() {
        return Err(inconsistent(format!(
            "seq {} is in events and in context",
            both.0
        )));
    }
    let runs: BTreeSet<RunId> = document
        .events
        .iter()
        .map(|event| read_entry(event).map(|record| record.run()))
        .collect::<Result<_, _>>()?;
    for event in &document.context {
        read_entry(event)?;
    }
    let ends = document
        .moments
        .iter()
        .flat_map(|moment| [moment.opened, moment.closed])
        .chain(
            document
                .messages
                .iter()
                .flat_map(|message| [message.sent, message.consumed]),
        )
        .chain(
            document
                .calls
                .iter()
                .flat_map(|call| [call.called, call.answered]),
        );
    for end in ends {
        match end {
            End::At(seq) if !events.contains(&seq) => {
                return Err(inconsistent(format!(
                    "an end at seq {} names no event",
                    seq.0
                )));
            }
            End::Outside(seq) if !context.contains(&seq) => {
                return Err(inconsistent(format!(
                    "an end outside the range at seq {} names no context line",
                    seq.0
                )));
            }
            End::At(_) | End::Outside(_) | End::Withheld | End::Pending | End::Missing => {}
        }
    }
    if let Some(cited) = traced_calls(document).find(|seq| !events.contains(seq)) {
        return Err(inconsistent(format!(
            "a commit's trace names seq {} as in the bundle, which is no event",
            cited.0
        )));
    }
    if let Some(member) = document
        .moments
        .iter()
        .flat_map(|moment| &moment.seqs)
        .find(|seq| !events.contains(seq))
    {
        return Err(inconsistent(format!(
            "a key moment names seq {}, which is no event",
            member.0
        )));
    }
    match document.runs.iter().find(|run| !runs.contains(&run.run)) {
        Some(stray) => Err(inconsistent(format!(
            "run {} has no event in the bundle",
            stray.run
        ))),
        None => Ok(()),
    }
}

/// The seqs every committed checkpoint's trace says are in `events`.
fn traced_calls(document: &Document) -> impl Iterator<Item = Decimal> + '_ {
    document
        .checkpoints
        .iter()
        .filter_map(|checkpoint| match &checkpoint.holds {
            Holds::Committed {
                trace: CallTrace::Traced { calls, .. },
                ..
            } => Some(calls),
            Holds::Committed { .. } | Holds::Pinned { .. } | Holds::Merged { .. } => None,
        })
        .flatten()
        .filter_map(|cited| match cited {
            Cited::At(seq) => Some(*seq),
            Cited::Elsewhere(_) | Cited::Withheld => None,
        })
}

/// The seqs of `entries`, refusing a list that is not strictly ascending.
fn ascending(section: &str, entries: &[Event]) -> Result<BTreeSet<Decimal>, AxError> {
    let mut seen = BTreeSet::new();
    let mut last: Option<Decimal> = None;
    for entry in entries {
        if last.is_some_and(|last| entry.seq <= last) {
            return Err(inconsistent(format!(
                "{section} are not in strictly ascending seq at {}",
                entry.seq.0
            )));
        }
        last = Some(entry.seq);
        seen.insert(entry.seq);
    }
    Ok(seen)
}

/// The record an entry's line holds, which must carry the entry's seq
/// and moment.
fn read_entry(entry: &Event) -> Result<kernel::EventRecord, AxError> {
    let record = match storage::read_line(entry.line.as_bytes()) {
        Ok(CheckedLine::Known(record)) => record,
        Ok(CheckedLine::IgnoredUnknown(_)) => {
            return Err(inconsistent(format!(
                "seq {} holds a line of a kind this build does not read",
                entry.seq.0
            )));
        }
        Err(fault) => {
            return Err(inconsistent(format!(
                "seq {} is not a ledger line: {fault:?}",
                entry.seq.0
            )));
        }
    };
    let moment = record.moment().map(|moment| Decimal(moment.value()));
    if record.seq().value() != entry.seq.0 || moment != entry.moment {
        return Err(inconsistent(format!(
            "the entry at seq {} does not match its line",
            entry.seq.0
        )));
    }
    Ok(record)
}

fn inconsistent(subject: String) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "check a playback bundle", subject).with_recovery(
        "export the bundle again with sprawling playback export; a bundle whose parts disagree \
         was changed after it was written",
    )
}
