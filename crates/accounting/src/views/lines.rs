// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `Views::rebuild` throws the whole thing
//! away and folds the ledger again to get the same bytes. That is
//! ARCHITECTURE.md section 9 shape 7, while `bin::assembly` is an
//! adapter - and a file holding two shapes is what section 9 says a split
//! looks like.
//!
//! **What it deliberately does not hold.** The plans are
//! `crate::plan_view`'s and are read through it; a second parse here
//! would be a second answer to "what is stuck and why", and only one of
//! them would be folding the records that say why. What waits in a room
//! is folded from signal records rather than read off a queue, because a
//! queue answers by being consumed and a view that consumed what it
//! showed would change the thing it reports on.

use std::path::Path;

use kernel::event::record::{AssetArchived, DiscardRestored, FileDiscarded};
use kernel::{Address, AxError, EventRecord, RunId};

/// The vendors this city knows by host, copied row for row from the
/// preset table (`crates/wire/Spec.lean` §8-51). A row the normaliser refuses
/// is a defect of the table, and the answer names itself unavailable
/// with the row's refusal rather than listing the rest as if whole.
pub(crate) fn known_hosts_answer() -> wire::Answer {
    let hosts = match gateway::known_hosts() {
        Ok(hosts) => hosts,
        Err(fault) => {
            return super::prepared::unavailable(format!("KnownHosts({})", fault.subject()));
        }
    };
    wire::Answer::KnownHosts(wire::KnownHostsAnswer {
        hosts: hosts
            .into_iter()
            .map(|row| wire::KnownHost {
                host: row.host.to_owned(),
                faces: row
                    .faces
                    .into_iter()
                    .map(|(dialect, base_url)| wire::KnownFace { dialect, base_url })
                    .collect(),
            })
            .collect(),
    })
}

/// The building a `pursuit_changed` record is about.
///
/// # Errors
/// Refuses a record with no address: the step it records belongs to no
/// building, and a fold that skipped it would keep whatever goal the
/// step changed.
pub fn pursued(record: &EventRecord) -> Result<Address, AxError> {
    record.addr().cloned().ok_or_else(|| {
        AxError::failure(
            kernel::AxCode::WireMismatch,
            "read a pursuit_changed line",
            format!("line {} names no building", record.seq().value()),
        )
        .with_recovery("replay with the build that wrote this record")
    })
}

/// Every building the city has, in reading order.
///
/// The plans themselves are `crate::plan_view`'s: reading them here as
/// well would be a second parse of the same file, and the two would
/// disagree the first time one of them was invalidated and the other was
/// not.
///
/// # Errors
/// Propagates `city::buildings`' refusal: a city root that cannot be
/// read is not a city with no buildings, and a caller answers
/// `Unavailable` rather than an empty list.
pub(crate) fn buildings_of(city_root: &Path) -> Result<Vec<Address>, AxError> {
    let mut found = city::buildings(city_root)?;
    found.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Ok(found)
}

/// The waiting row a `signal_enqueued` line adds, and the room it waits
/// in, read through the struct its writer wrote.
///
/// # Errors
/// Refuses a line this build cannot read as a signal: skipping it would
/// leave a waiting signal out of the view.
pub(crate) fn signal_line(record: &EventRecord) -> Result<(Address, wire::SignalLine), AxError> {
    let signal = collab::Signal::from_payload(record.data())?;
    Ok((
        signal.room().clone(),
        wire::SignalLine {
            id: signal.id().as_str().to_owned(),
            kind: signal.kind().as_str().to_owned(),
            from: signal.from().to_owned(),
            at: record.t(),
            first_line: wire::text(signal.payload().as_map().get("text"))
                .and_then(|said| said.lines().next().map(str::to_owned)),
        },
    ))
}

/// The rows one `file_discarded` record states: one per path, each with
/// the record's way back. A record this version cannot read states no
/// rows: a discard view that refused the whole fold over one old record
/// would lose every row it can read.
pub(crate) fn discard_lines(record: &EventRecord) -> Vec<wire::DiscardLine> {
    let Ok(FileDiscarded { paths, restoration }) = record.data().read() else {
        return Vec::new();
    };
    paths
        .into_iter()
        .map(|path| wire::DiscardLine {
            path,
            restoration: restoration.clone(),
            at: record.t(),
            restored: false,
        })
        .collect()
}

/// The paths one `discard_restored` record put back; none when this
/// version cannot read it.
pub(crate) fn restored_paths(record: &EventRecord) -> Vec<String> {
    record
        .data()
        .read::<DiscardRestored>()
        .map_or_else(|_| Vec::new(), |restored| restored.paths)
}

/// One shelf entry, as the registry shows it. `None` for a record with
/// no room or one this version cannot read.
pub(crate) fn registry_line(record: &EventRecord) -> Option<wire::RegistryLine> {
    let AssetArchived { kind, subject, .. } = record.data().read().ok()?;
    Some(wire::RegistryLine {
        addr: record.addr().cloned()?,
        kind,
        subject,
        at: record.t(),
    })
}

pub(crate) fn summarize(run: RunId, hot: &storage::RunHot) -> wire::RunSummary {
    wire::RunSummary {
        run,
        who: hot.who.clone(),
        frozen: matches!(hot.phase, storage::RunPhase::Frozen),
        last_seq: hot.last_seq,
        last_kind: hot.last_kind,
        addr: hot.addr.clone(),
        started: hot.started,
        completion: hot.completion.clone(),
        pr: hot.pr.clone(),
        ask: hot.ask.clone(),
        task: hot.task.clone(),
        goal: hot.goal.clone(),
        waiting: hot.waiting.as_ref().map(|waiting| wire::Waiting {
            on: waiting.on.clone(),
            until: waiting.until,
        }),
    }
}
