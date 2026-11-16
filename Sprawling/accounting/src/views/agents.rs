// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use agent_protocols::{AgentEntry, AgentSource, Pin};

use super::holding::Views;
use super::prepared::{Prepared, unavailable_because};

pub struct CatalogAsk {
    city_root: PathBuf,
    place: Option<fn(&agent_protocols::SetUpDir) -> Option<PathBuf>>,
    offered: Option<crate::offered::Offered>,
}

pub(super) fn catalog_ask(views: &Views) -> Prepared {
    Prepared::Agents(CatalogAsk {
        city_root: views.city_root.clone(),
        place: views.reach.places,
        offered: views.offered.clone(),
    })
}

impl CatalogAsk {

    pub(super) fn answer(self) -> wire::Answer {
        match crate::roster::roster(&self.city_root) {
            Ok(roster) => {
                let detected = self
                    .place
                    .map(|place| evidence(&roster, place))
                    .unwrap_or_default();
                if let Some(offered) = &self.offered {
                    offered.remember(&detected);
                }
                catalog_of(&roster, &detected)
            }
            Err(stopped) => unavailable_because("AgentCatalog".to_owned(), &stopped),
        }
    }
}

fn catalog_of(roster: &agent_protocols::Roster, detected: &[AgentEntry]) -> wire::Answer {
    wire::Answer::AgentCatalog(Box::new(wire::AgentCatalogAnswer {
        detected: detected.iter().map(offer_of).collect(),
        catalog: roster.catalog().entries.iter().map(offer_of).collect(),
        added: roster.rows().map(line_of).collect(),
        snapshot: wire::CatalogSnapshot {
            date: roster.catalog().date.clone(),
            etag: roster.catalog().etag.clone(),
        },
    }))
}

fn evidence(
    roster: &agent_protocols::Roster,
    place: fn(&agent_protocols::SetUpDir) -> Option<PathBuf>,
) -> Vec<AgentEntry> {
    let configs: Vec<String> = agent_protocols::CLIENT_CONFIGS
        .iter()
        .filter_map(place)

        .flat_map(std::fs::read_to_string)
        .collect();
    agent_protocols::detected(
        roster,
        |dir| place(dir).is_some_and(|at| at.is_dir()),
        &configs,
    )
}

pub(super) fn pasted_ask(views: &Views, text: &str) -> Prepared {
    match agent_protocols::pasted(text) {
        Ok(entry) => {
            if let Some(offered) = &views.offered {
                offered.remember([&entry]);
            }
            Prepared::Held(wire::Answer::AgentSpec(Box::new(offer_of(&entry))))
        }
        Err(refusal) => Prepared::Refused(refusal),
    }
}

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

        login: Vec::new(),
        spec_digest: entry.launch.digest(),
    }
}

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
