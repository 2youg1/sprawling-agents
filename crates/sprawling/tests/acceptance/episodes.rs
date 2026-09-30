// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One episode per built-in tool: the call the script makes, and what
//! the tool's own SPEC says the answer is.
//!
//! A tool the workbench registers without an episode here is named by
//! the catalogue tests as offered and never called. The verdicts read
//! the part of a result that belongs to the tool's contract rather than
//! the whole result, so a field added for every tool later does not
//! turn them red.

use std::path::Path;

use kernel::RunId;
use kernel::event::record::ToolAnswer;

use crate::city::History;
use crate::script::Step;

/// Where one catalogue test works: the building, the room the task is
/// sent to, and a room beside it with a resident.
pub(crate) struct Setup {
    pub(crate) building: &'static str,
    pub(crate) room: &'static str,
    pub(crate) neighbour: &'static str,
}

/// A building that builds, with a tree of its own for each room.
pub(crate) const LAB: Setup = Setup {
    building: "lab",
    room: "lab/lead",
    neighbour: "lab/other",
};

/// City Hall, which plans (city-SPEC.md 8-22).
pub(crate) const HALL: Setup = Setup {
    building: "hall",
    room: "hall/mayor",
    neighbour: "hall/clerk",
};

/// What a verdict may look at once the dispatch has landed.
pub(crate) struct Observed<'a> {
    pub(crate) setup: &'a Setup,
    pub(crate) city: &'a Path,
    pub(crate) history: &'a History,
    /// The run the script drove.
    pub(crate) lead: RunId,
    pub(crate) answer: &'a ToolAnswer,
}

pub(crate) struct Episode {
    pub(crate) step: Step,
    pub(crate) holds: fn(&Observed<'_>) -> Result<(), String>,
}

/// The episodes for a building that builds, in the order they run.
pub(crate) fn for_builders(setup: &Setup) -> Vec<Episode> {
    vec![status(setup)]
}

/// The episodes for City Hall, in the order they run.
pub(crate) fn for_city_hall(setup: &Setup) -> Vec<Episode> {
    vec![status(setup)]
}

fn status(_setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "status",
            args: serde_json::json!({}),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            let line = format!("addr: {}", seen.setup.room);
            if text.lines().any(|at| at == line) {
                Ok(())
            } else {
                Err(format!("no `{line}` line in {text:?}"))
            }
        },
    }
}

/// The result of a call that answered.
fn answered(answer: &ToolAnswer) -> Result<&serde_json::Map<String, serde_json::Value>, String> {
    match answer {
        ToolAnswer::Answered { result } => Ok(result.as_map()),
        ToolAnswer::Failed { error } => Err(format!("failed: {:?}", error.as_map())),
    }
}

/// The `text` field of a call that answered.
fn answered_text(answer: &ToolAnswer) -> Result<&str, String> {
    answered(answer)?
        .get("text")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "the result carries no text".to_owned())
}
