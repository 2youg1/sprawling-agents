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
//! turn them red. A verdict that fails is either an episode that
//! misreads its tool's SPEC, fixed here, or a product that disagrees
//! with its SPEC, fixed in the crate that owns the tool; what the run
//! happened to answer is never copied into a verdict.

use std::path::Path;

use kernel::event::record::ToolAnswer;
use kernel::{EventKind, RunId};
use serde_json::{Map, Value, json};

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
    /// The building's `RULES.toml` as it stood before the dispatch.
    pub(crate) rules_before: &'a [u8],
    pub(crate) answer: &'a ToolAnswer,
}

pub(crate) struct Episode {
    pub(crate) step: Step,
    pub(crate) holds: fn(&Observed<'_>) -> Result<(), String>,
}

/// Written into the notes and looked for again, so `read` and `search`
/// are judged on bytes this run put there.
const MARK: &str = "kiln-temperature-1280";

/// The episodes for a building that builds, in the order they run.
/// `succeed` is last because the successor starts only when this run
/// ends.
pub(crate) fn for_builders(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([
        pr_open(),
        rules(),
        exec(),
        delegate(setup),
        workshop(setup),
        succeed(),
    ]);
    episodes
}

/// The episodes for City Hall, in the order they run. It has no tree of
/// its own, so `pr` lists rather than opens; it has no `exec`,
/// `delegate` or `workshop`, and it has `city`.
pub(crate) fn for_city_hall(setup: &Setup) -> Vec<Episode> {
    let mut episodes = shared(setup);
    episodes.extend([pr_list(), rules(), city(), succeed()]);
    episodes
}

/// The episodes every building's table shares.
fn shared(setup: &Setup) -> Vec<Episode> {
    vec![
        status(),
        neighbours(),
        edit(setup),
        read(setup),
        search(),
        archive(),
        goal(setup),
        plan(),
        signal(setup),
    ]
}

/// The notes file the lead creates in its own room.
fn notes(setup: &Setup) -> String {
    format!("{}/notes.md", setup.room)
}

fn status() -> Episode {
    Episode {
        step: Step {
            tool: "status",
            args: json!({}),
        },
        holds: |seen| {
            let line = format!("addr: {}", seen.setup.room);
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().any(|at| at == line),
                format!("no `{line}` line in {text:?}"),
            )
        },
    }
}

fn neighbours() -> Episode {
    Episode {
        step: Step {
            tool: "neighbours",
            args: json!({}),
        },
        holds: |seen| {
            let entry = format!("- {}:", seen.setup.neighbour);
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().any(|at| at.starts_with(&entry)),
                format!("{} is not listed in {text:?}", seen.setup.neighbour),
            )
        },
    }
}

fn edit(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "edit",
            args: json!({
                "path": notes(setup),
                "base_version": "new",
                "old": "",
                "new": format!("# Notes\n\n{MARK}\n"),
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("base_version") == Some(&json!("new")),
                format!("a creation answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .wrote(seen.lead, EventKind::CheckpointCommitted),
                "no checkpoint was committed after the write".to_owned(),
            )
        },
    }
}

fn read(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "read",
            args: json!({ "path": notes(setup) }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.contains(MARK),
                format!("the notes read back as {text:?}"),
            )
        },
    }
}

fn search() -> Episode {
    Episode {
        step: Step {
            tool: "search",
            args: json!({ "text": MARK }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let at = notes(seen.setup);
            let found = result
                .get("matches")
                .and_then(Value::as_array)
                .is_some_and(|hits| hits.iter().any(|hit| hit.get("path") == Some(&json!(at))));
            ensure(found, format!("{at} is not among {result:?}"))
        },
    }
}

fn archive() -> Episode {
    Episode {
        step: Step {
            tool: "archive",
            args: json!({
                "action": "record",
                "kind": "decision",
                "text": "The kiln is fired at night.",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("recorded") == Some(&json!(true))
                    && result.get("kind") == Some(&json!("decision")),
                format!("the record answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::AssetArchived, "The kiln is fired at night."),
                "no asset_archived line carries the decision".to_owned(),
            )
        },
    }
}

fn goal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "goal",
            args: json!({
                "statement": "keep the notes",
                "paths": [notes(setup)],
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("registered") == Some(&json!(true)),
                format!("the goal answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::GoalRegistered, "keep the notes"),
                "no goal_registered line carries the statement".to_owned(),
            )
        },
    }
}

fn plan() -> Episode {
    Episode {
        step: Step {
            tool: "plan",
            args: json!({ "action": "claim", "node": "1" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("item") == Some(&json!("wire the kiln")),
                format!("the claim answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::RoadmapClaimed, "\"node\":\"1\""),
                "no roadmap_claimed line names node 1".to_owned(),
            )
        },
    }
}

fn signal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "signal",
            args: json!({
                "action": "send",
                "to": setup.neighbour,
                "text": "the kiln is free",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("queued") == Some(&json!(true))
                    && result.get("to") == Some(&json!(seen.setup.neighbour)),
                format!("the send answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::SignalEnqueued, seen.setup.neighbour),
                format!("no signal_enqueued line names {}", seen.setup.neighbour),
            )
        },
    }
}

fn pr_open() -> Episode {
    Episode {
        step: Step {
            tool: "pr",
            args: json!({ "action": "open" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let Some(branch) = result.get("branch").and_then(Value::as_str) else {
                return Err(format!("the request names no branch: {result:?}"));
            };
            ensure(
                seen.history.says(EventKind::PrOpened, branch),
                format!("no pr_opened line names {branch}"),
            )
        },
    }
}

fn pr_list() -> Episode {
    Episode {
        step: Step {
            tool: "pr",
            args: json!({ "action": "list" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("requests").is_some_and(Value::is_array),
                format!("the list answered {result:?}"),
            )
        },
    }
}

/// Refused at the effect layer: a run does not change what governs it
/// (city-SPEC.md 8-2b). Judged by the file staying as it was.
fn rules() -> Episode {
    Episode {
        step: Step {
            tool: "rules",
            args: json!({ "op": "propose", "text": "confidential = true\nwrite = \"everything\"\n" }),
        },
        holds: |seen| {
            failed(seen.answer)?;
            let path = ::city::rules_path(seen.city, &address(seen.setup.building));
            let now = std::fs::read(&path).unwrap_or_default();
            ensure(
                now == seen.rules_before,
                format!("{} changed under a refused proposal", path.display()),
            )
        },
    }
}

/// A shell is off unless a building turns it on, so the shell arm has
/// nothing to run (runtime-SPEC.md, `exec`).
fn exec() -> Episode {
    Episode {
        step: Step {
            tool: "exec",
            args: json!({ "arm": { "shell": { "text": "echo hi" } } }),
        },
        holds: |seen| {
            let error = failed(seen.answer)?;
            ensure(
                error.get("code") == Some(&json!("E_TOOL_UNAVAILABLE")),
                format!("the shell arm answered {error:?}"),
            )
        },
    }
}

fn delegate(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "delegate",
            args: json!({
                "room": format!("{}/helper", setup.building),
                "task": "measure the kiln",
                "goal": "a number, then stop",
            }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            started_under(seen, &format!("{}/helper", seen.setup.building))
        },
    }
}

fn workshop(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "workshop",
            args: json!({
                "op": "lay_out",
                "nodes": [{
                    "room": format!("{}/reader", setup.building),
                    "goal": "read the notes",
                    "done_check": "the notes were read",
                    "stop": "when the notes were read",
                    "depends_on": [],
                }],
            }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            started_under(seen, &format!("{}/reader", seen.setup.building))
        },
    }
}

fn succeed() -> Episode {
    Episode {
        step: Step {
            tool: "succeed",
            args: json!({ "reason": "the window is full" }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            ensure(
                seen.history
                    .started()
                    .iter()
                    .any(|started| started.record.predecessor == Some(seen.lead)),
                "no run started as this run's successor".to_owned(),
            )
        },
    }
}

/// Refused at the effect layer: a run does not raise a building
/// (city-SPEC.md 8-23). Judged by the building not existing.
fn city() -> Episode {
    Episode {
        step: Step {
            tool: "city",
            args: json!({ "action": "raise", "name": "kiln" }),
        },
        holds: |seen| {
            failed(seen.answer)?;
            ensure(
                !seen.city.join("kiln").exists(),
                "a refused raise left a building behind".to_owned(),
            )
        },
    }
}

/// Whether a run started at `room` with the lead as its parent.
fn started_under(seen: &Observed<'_>, room: &str) -> Result<(), String> {
    ensure(
        seen.history.started().iter().any(|started| {
            started.record.parent == Some(seen.lead)
                && started.addr.as_ref().map(kernel::Address::as_str) == Some(room)
        }),
        format!("no run started at {room} under the lead"),
    )
}

fn address(raw: &str) -> kernel::Address {
    kernel::Address::parse(raw).unwrap()
}

fn ensure(holds: bool, otherwise: String) -> Result<(), String> {
    if holds { Ok(()) } else { Err(otherwise) }
}

/// The result of a call that answered.
fn answered(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Answered { result } => Ok(result.as_map()),
        ToolAnswer::Failed { error } => Err(format!("failed: {:?}", error.as_map())),
    }
}

/// The error of a call that failed.
fn failed(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Failed { error } => Ok(error.as_map()),
        ToolAnswer::Answered { result } => Err(format!("answered: {:?}", result.as_map())),
    }
}

/// The `text` field of a call that answered.
fn answered_text(answer: &ToolAnswer) -> Result<&str, String> {
    answered(answer)?
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "the result carries no text".to_owned())
}
