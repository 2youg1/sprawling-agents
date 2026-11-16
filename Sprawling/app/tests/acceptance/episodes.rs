// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::ToolAnswer;
use kernel::RunId;
use serde_json::{Map, Value};

use crate::city::History;
use crate::script::Step;

pub(crate) struct Setup {
    pub(crate) building: &'static str,
    pub(crate) room: &'static str,
    pub(crate) neighbour: &'static str,
}

pub(crate) const LAB: Setup = Setup {
    building: "lab",
    room: "lab/lead",
    neighbour: "lab/other",
};

pub(crate) const HALL: Setup = Setup {
    building: "hall",
    room: "hall/mayor",
    neighbour: "hall/clerk",
};

pub(crate) struct Observed<'a> {
    pub(crate) setup: &'a Setup,
    pub(crate) city: &'a Path,
    pub(crate) history: &'a History,

    pub(crate) lead: RunId,

    pub(crate) rules_before: &'a [u8],
    pub(crate) answer: &'a ToolAnswer,
}

pub(crate) struct Episode {
    pub(crate) step: Step,
    pub(crate) holds: fn(&Observed<'_>) -> Result<(), String>,
}

mod catalog;
mod hearing;

pub(crate) use catalog::lay_draft;
pub(crate) use hearing::{OPEN_LAB, choose, read_request, stored_for};

pub(crate) fn for_builders(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([
        catalog::pr_open(),
        catalog::rules(),
        catalog::exec(),
        catalog::delegate(setup),
        catalog::workshop(setup),
        crate::web_search::episode(),
        catalog::succeed(),
    ]);
    episodes
}

pub(crate) fn for_city_hall(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([
        catalog::pr_list(),
        catalog::rules(),
        catalog::city(),
        crate::web_search::episode(),
        catalog::succeed(),
    ]);
    episodes
}

fn shared(setup: &Setup) -> Vec<Episode> {
    vec![
        catalog::status(),
        catalog::neighbours(),
        catalog::edit(setup),
        catalog::read(setup),
        catalog::search(),
        catalog::archive(),
        catalog::goal(setup),
        catalog::plan(),
        catalog::signal(setup),
        crate::playback::episode(),
        catalog::proposal(setup),
        catalog::detail(),
        catalog::run_itself(),
        catalog::list("sprawling"),
        catalog::list("record"),
        catalog::list("web"),
    ]
}

pub(super) fn notes(setup: &Setup) -> String {
    format!("{}/notes.md", setup.room)
}

pub(super) fn started_under(seen: &Observed<'_>, room: &str) -> Result<(), String> {
    ensure(
        seen.history.started().iter().any(|started| {
            started.record.parent == Some(seen.lead)
                && started.addr.as_ref().map(kernel::Address::as_str) == Some(room)
        }),
        format!("no run started at {room} under the lead"),
    )
}

pub(super) fn address(raw: &str) -> kernel::Address {
    kernel::Address::parse(raw).unwrap()
}

pub(super) fn ensure(holds: bool, otherwise: String) -> Result<(), String> {
    if holds { Ok(()) } else { Err(otherwise) }
}

pub(super) fn answered(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Answered { result } => Ok(result.as_map()),
        ToolAnswer::Failed { error } => Err(format!("failed: {:?}", error.as_map())),
    }
}

pub(super) fn failed(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Failed { error } => Ok(error.as_map()),
        ToolAnswer::Answered { result } => Err(format!("answered: {:?}", result.as_map())),
    }
}

pub(crate) fn answered_text(answer: &ToolAnswer) -> Result<&str, String> {
    answered(answer)?
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "the result carries no text".to_owned())
}
