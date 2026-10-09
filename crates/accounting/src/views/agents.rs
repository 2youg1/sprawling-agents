// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ACP page's two questions: the agent catalog with the agents this
//! city added, and one pasted agent (`crates/accounting/spec/Views.lean`,
//! `crates/wire/spec/Answer/Agents.lean` §8-90).

use std::path::Path;

use agent_protocols::{AgentEntry, AgentSource, Pin};

use super::holding::Views;
use super::lines::HarnessReach;
use super::prepared::{Prepared, unavailable_because};

/// The catalog page's paths, copied out so the files are read with the
/// snapshot let go.
pub(super) fn catalog_ask(views: &Views) -> Prepared {
    Prepared::Agents(views.city_root.clone(), views.reach.programs)
}

/// The catalog page, read from the shipped snapshot and the city's own
/// `CONFIG.toml`, with the agents this machine shows evidence of when the
/// served city handed in where to look; nothing is run.
pub(super) fn catalog_answer(city_root: &Path, reach: Option<HarnessReach>) -> wire::Answer {
    match crate::roster::roster(city_root) {
        Ok(roster) => wire::Answer::AgentCatalog(Box::new(wire::AgentCatalogAnswer {
            detected: reach
                .map(|reach| evidence(&roster, reach))
                .unwrap_or_default()
                .iter()
                .map(offer_of)
                .collect(),
            catalog: roster.catalog().entries.iter().map(offer_of).collect(),
            added: roster.rows().map(line_of).collect(),
            snapshot: wire::CatalogSnapshot {
                date: roster.catalog().date.clone(),
                etag: roster.catalog().etag.clone(),
            },
        })),
        Err(stopped) => unavailable_because("AgentCatalog".to_owned(), &stopped),
    }
}

/// What this machine shows: the other clients' files that exist and read,
/// and the vendor directories that exist.
fn evidence(roster: &agent_protocols::Roster, reach: HarnessReach) -> Vec<AgentEntry> {
    let configs: Vec<String> = agent_protocols::CLIENT_CONFIGS
        .iter()
        .filter_map(reach.place)
        // A client that is not installed has no file, and one that cannot
        // be read shows nothing: evidence only orders and hints, so a file
        // that does not read is no evidence rather than a refused page.
        .flat_map(std::fs::read_to_string)
        .collect();
    let place = reach.place;
    agent_protocols::detected(
        roster,
        |dir| place(dir).is_some_and(|at| at.is_dir()),
        &configs,
    )
}

/// One pasted agent as its consent card shows it, or the paste's refusal.
/// The grammar is pure and reads nothing, so it is settled with the
/// snapshot held.
pub(super) fn pasted_ask(text: &str) -> Prepared {
    match agent_protocols::pasted(text) {
        Ok(entry) => Prepared::Held(wire::Answer::AgentSpec(Box::new(offer_of(&entry)))),
        Err(refusal) => Prepared::Refused(refusal),
    }
}

/// One entry as its consent card shows it.
fn offer_of(entry: &AgentEntry) -> wire::AgentOffer {
    wire::AgentOffer {
        id: entry.id.as_str().to_owned(),
        name: entry.name.clone(),
        source: source_of(entry.source),
        launch_preview: entry.launch.preview(),
        version: entry.version.clone(),
        pinned: pin_of(entry.launch.pin()),
        licence: entry.licence.clone(),
        env_names: entry
            .launch
            .env
            .iter()
            .map(|(name, _)| name.clone())
            .collect(),
        // Which logins an agent offers is known only once it answered
        // `initialize`, which the city does not send before consent.
        login: Vec::new(),
        spec_digest: entry.launch.digest(),
    }
}

/// One added agent. No session has been opened by this page, so its login
/// is unasked; the rooms it sits in are not read yet.
fn line_of(entry: &AgentEntry) -> wire::AgentLine {
    wire::AgentLine {
        id: entry.id.as_str().to_owned(),
        name: entry.name.clone(),
        source: source_of(entry.source),
        version: entry.version.clone(),
        pinned: pin_of(entry.launch.pin()),
        login_state: wire::LoginState::Unasked,
        auth_methods: Vec::new(),
        seated_in: Vec::new(),
    }
}

fn source_of(source: AgentSource) -> wire::AgentSource {
    match source {
        AgentSource::Registry => wire::AgentSource::Registry,
        AgentSource::Detected => wire::AgentSource::Detected,
        AgentSource::Pasted => wire::AgentSource::Pasted,
    }
}

fn pin_of(pin: Pin) -> wire::PinState {
    match pin {
        Pin::Exact => wire::PinState::Exact,
        Pin::Floating => wire::PinState::Floating,
        Pin::Unknown => wire::PinState::Unknown,
    }
}
