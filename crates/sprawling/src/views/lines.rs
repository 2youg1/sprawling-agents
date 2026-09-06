// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `rebuild_views` throws the whole thing
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

use kernel::{Address, EventRecord, RunId};

// Where a city keeps its ledger and how a building reads off disk are
// `bin::assembly`'s: it forms the city that laid them out. Borrowed
// rather than copied, so "where the ledger lives" keeps one answer.
/// The settings page's read of the endpoint book.
pub(crate) fn endpoints_answer(book: &gateway::EndpointBook) -> channels::EndpointsAnswer {
    let endpoints = book
        .endpoints()
        .map(|endpoint| channels::EndpointSummary {
            name: endpoint.name.clone(),
            base_url: endpoint.base_url.clone(),
            dialect: endpoint.dialect,
            models: endpoint.models.clone(),
            local: endpoint.is_local(),
            has_credential: endpoint.has_credential(),
        })
        .collect();
    let chosen = book
        .choices()
        .map(|(tag, endpoint, entry)| channels::ChosenSummary {
            tag,
            endpoint: endpoint.to_owned(),
            model: entry.id.clone(),
            max_output_tokens: entry.max_output_tokens,
        })
        .collect();
    channels::EndpointsAnswer { endpoints, chosen }
}

/// One clause a person reads: what the city is doing about its pursuit.
///
/// The one wording, so the page, the console and a log line all say the
/// same thing about the same verdict.
pub(crate) fn verdict_line(verdict: kernel::PursuitVerdict) -> String {
    match verdict {
        kernel::PursuitVerdict::Work { next } => format!("working on {next}"),
        kernel::PursuitVerdict::Waiting { in_flight } => {
            format!("waiting for {in_flight} run(s) still going")
        }
        kernel::PursuitVerdict::Paused => "paused".to_owned(),
        kernel::PursuitVerdict::Finished => {
            "finished: nothing is ready and nobody is working".to_owned()
        }
    }
}

/// What one `pursuit_changed` record says.
///
/// `None` for a record this build cannot read as one, which a view skips
/// rather than inventing a goal for.
pub(crate) fn pursuit_from(
    record: &EventRecord,
) -> Option<(Address, Option<(String, kernel::PursuitState)>)> {
    let map = record.data().as_map();
    let addr = record.addr()?.clone();
    let step = map.get("step")?.as_str()?;
    let goal = map
        .get("goal")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let held = match step {
        "set" => Some((goal, kernel::PursuitState::Running)),
        "pause" => Some((goal, kernel::PursuitState::Paused)),
        "resume" => Some((goal, kernel::PursuitState::Running)),
        "clear" => None,
        _ => return None,
    };
    Some((addr, held))
}

/// Every building the city has, in reading order.
///
/// The plans themselves are `crate::plan_view`'s: reading them here as
/// well would be a second parse of the same file, and the two would
/// disagree the first time one of them was invalidated and the other was
/// not.
pub(crate) fn buildings_of(city_root: &Path) -> Vec<Address> {
    let mut found = city::buildings(city_root).unwrap_or_default();
    found.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    found
}

/// One signal, as a room's queue would show it. `None` for a record
/// this version cannot read as a signal: a view skips what it cannot
/// read rather than inventing a row for it.
pub(crate) fn signal_line(record: &EventRecord) -> Option<(Address, channels::SignalLine)> {
    let map = record.data().as_map();
    let text = |key: &str| {
        map.get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let room = Address::parse(&text("room")?).ok()?;
    Some((
        room,
        channels::SignalLine {
            id: text("id")?,
            kind: text("kind").unwrap_or_else(|| "signal".to_owned()),
            from: text("from").unwrap_or_default(),
            at: record.t(),
        },
    ))
}

/// The rows one discard record states. A record carries the paths it
/// discarded and one restoration per path.
pub(crate) fn discard_lines(record: &EventRecord) -> Vec<channels::DiscardLine> {
    let map = record.data().as_map();
    // The plan travels as itself. It was written by serialising a
    // `Restoration`, so it reads back as one; a scheme this build cannot
    // name comes through as `None` and the row still appears.
    let restoration = map
        .get("restoration")
        .cloned()
        .and_then(|way| serde_json::from_value::<channels::Restoration>(way).ok());
    map.get("paths")
        .and_then(serde_json::Value::as_array)
        .map(|paths| {
            paths
                .iter()
                .filter_map(|path| {
                    Some(channels::DiscardLine {
                        path: path.as_str()?.to_owned(),
                        restoration: restoration.clone(),
                        at: record.t(),
                        restored: false,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn registry_line(record: &EventRecord) -> Option<channels::RegistryLine> {
    let map = record.data().as_map();
    let text = |key: &str| map.get(key).and_then(serde_json::Value::as_str);
    Some(channels::RegistryLine {
        addr: record.addr().cloned()?,
        kind: text("kind").unwrap_or("fact").to_owned(),
        subject: text("subject").unwrap_or_default().to_owned(),
        at: record.t(),
    })
}

pub(crate) fn summarize(run: RunId, hot: &memory::RunHot) -> channels::RunSummary {
    channels::RunSummary {
        run,
        who: hot.who.clone(),
        frozen: matches!(hot.phase, memory::RunPhase::Frozen),
        last_seq: hot.last_seq,
        last_kind: hot.last_kind,
    }
}
