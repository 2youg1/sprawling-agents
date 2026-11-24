// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::{AssetArchived, DiscardRestored, FileDiscarded};
use kernel::{Address, AxError, EventRecord, RunId};

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

pub(crate) fn buildings_of(city_root: &Path) -> Result<Vec<Address>, AxError> {
    let mut found = city::buildings(city_root)?;
    found.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    Ok(found)
}

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

pub(crate) fn restored_paths(record: &EventRecord) -> Vec<String> {
    record
        .data()
        .read::<DiscardRestored>()
        .map_or_else(|_| Vec::new(), |restored| restored.paths)
}

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
        session_began: hot.session_began,
        waiting: hot.waiting.as_ref().map(|waiting| wire::Waiting {
            on: waiting.on.clone(),
            until: waiting.until,
        }),
    }
}
