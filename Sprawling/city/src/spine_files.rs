// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError};

use crate::policy::rules_path;

pub(crate) mod hall;

pub(crate) mod blank;
use blank::{empty_roadmap, is_blank_form};

pub use hall::{CLERK_FILE, MAYOR_FILE, hall_discipline, hall_identity_path};

pub const ROADMAP_FILE: &str = kernel::ROADMAP_FILE;

pub const MEMO_FILE: &str = "Memo.md";

pub const HANDOFF_FILE: &str = kernel::layout::HANDOFF_FILE;

pub const SPEC_FILE: &str = "SPEC.md";

pub const CITY_FILE: &str = "City.md";

pub(crate) const CITY_TEMPLATE: &str = include_str!("../templates/City.md");

pub const AGENTS_FILE: &str = "AGENTS.md";

const ROADMAP_TEMPLATE: &str = include_str!("../templates/Roadmap.md");
const MEMO_TEMPLATE: &str = include_str!("../templates/Memo.md");
const HANDOFF_TEMPLATE: &str = include_str!("../templates/Handoff.md");

const SPEC_TEMPLATE: &str = include_str!("../templates/SPEC.md");

const JOB_TEMPLATE: &str = include_str!("../templates/JOB.md");
const FROM_PLACEHOLDER: &str = "<fill-from>";
const TASK_PLACEHOLDER: &str = "<fill-task>";
const GOAL_PLACEHOLDER: &str = "<fill-goal>";
use crate::building::template::NAME_PLACEHOLDER;

const PROJECT_PLACEHOLDER: &str = "<project name>";

pub struct JobBrief<'a> {
    pub from: &'a str,

    pub task: &'a str,

    pub goal: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunBrief {
    Job { text: String },

    Principal,
}

const PRINCIPAL_BRIEF: &str = "No task file was written for this session. You are working with the \
     User directly: what to do arrives in the conversation, and you \
     answer there.\n";

impl RunBrief {
    #[must_use]
    pub fn segment_text(&self) -> &str {
        match self {
            RunBrief::Job { text } => text,
            RunBrief::Principal => PRINCIPAL_BRIEF,
        }
    }
}

pub fn write_brief(
    city_root: &Path,
    addr: &Address,
    brief: &JobBrief<'_>,
) -> Result<RunBrief, AxError> {
    if brief.goal.trim().is_empty() {
        return Ok(RunBrief::Principal);
    }
    let text = write_job(city_root, addr, brief)?;
    Ok(RunBrief::Job { text })
}

#[must_use]
pub fn handoff_path(city_root: &Path, room: &Address) -> PathBuf {
    kernel::layout::CityLayout::new(city_root).handoff(room)
}

pub fn clear_handoff(city_root: &Path, room: &Address) -> Result<(), AxError> {
    let path = handoff_path(city_root, room);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "empty a room's handoff slot",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "a new session leaves the last session's handoff behind only once this \
             file is gone; make it removable, then start the session again",
        )),
    }
}

pub(crate) fn lay_out_handoff(room_dir: &Path, room: &Address) -> Result<(), AxError> {
    write_new(
        &room_dir.join(HANDOFF_FILE),
        &HANDOFF_TEMPLATE.replace(NAME_PLACEHOLDER, room.as_str()),
    )
}

pub fn handoff(city_root: &Path, room: &Address) -> Result<Option<String>, AxError> {
    let path = handoff_path(city_root, room);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(AxError::failure(
                AxCode::StorageFatal,
                "read a room's handoff",
                format!("{}: {err}", path.display()),
            )
            .with_recovery(
                "the next session is assembled from this file; \
                 make it readable, then dispatch again",
            ));
        }
    };
    if is_blank_form(&text) {
        return Ok(None);
    }
    Ok(Some(text))
}

pub(crate) fn lay_out(building_root: &Path, addr: &Address) -> Result<(), AxError> {
    std::fs::create_dir_all(building_root).map_err(|err| storage(building_root, &err))?;
    let name = addr.as_str();
    write_new(
        &building_root.join(ROADMAP_FILE),
        &empty_roadmap(&ROADMAP_TEMPLATE.replace(NAME_PLACEHOLDER, name)),
    )?;
    write_new(
        &building_root.join(MEMO_FILE),
        &MEMO_TEMPLATE.replace(NAME_PLACEHOLDER, name),
    )?;
    write_new(
        &building_root.join(SPEC_FILE),
        &SPEC_TEMPLATE.replace(PROJECT_PLACEHOLDER, name),
    )?;
    Ok(())
}

#[must_use]
pub fn job_path(city_root: &Path, addr: &Address) -> PathBuf {
    kernel::layout::CityLayout::new(city_root).job(addr)
}

#[must_use]
pub fn roadmap_path(city_root: &Path, building_addr: &Address) -> PathBuf {
    kernel::layout::CityLayout::new(city_root)
        .scope(building_addr)
        .join(ROADMAP_FILE)
}

pub fn roadmap(city_root: &Path, building_addr: &Address) -> Result<String, AxError> {
    let path = roadmap_path(city_root, building_addr);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read a building's plan",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "the plan is shared ground and a run may not claim on it unread; \
             make the file readable, then dispatch again",
        )),
    }
}

fn filled(brief: &JobBrief<'_>) -> String {
    JOB_TEMPLATE
        .replace(TASK_PLACEHOLDER, &kernel::event::escape_markup(brief.task))
        .replace(GOAL_PLACEHOLDER, &kernel::event::escape_markup(brief.goal))
        .replace(FROM_PLACEHOLDER, brief.from)
}

pub fn write_job(
    city_root: &Path,
    addr: &Address,
    brief: &JobBrief<'_>,
) -> Result<String, AxError> {
    let path = job_path(city_root, addr);
    let text = filled(brief);
    crate::document::replace(&path, text.as_bytes())?;
    Ok(text)
}

pub fn city_prompt_path(city_root: &Path, building: &Address) -> Result<PathBuf, AxError> {
    let own = crate::policy::governed(city_root, building).join(CITY_FILE);
    match std::fs::symlink_metadata(&own) {
        Ok(_) => Ok(own),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(city_root.join(CITY_FILE)),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "find the City.md that governs a building",
            format!("{}: {err}", own.display()),
        )
        .with_recovery("make the building's reserved directory readable, then dispatch again")),
    }
}

pub fn norms(city_root: &Path, addr: &Address) -> Result<Vec<PathBuf>, AxError> {
    let building = crate::building::Building::of(addr)?;
    let mut out = vec![city_prompt_path(city_root, building.addr())?];
    let rules = rules_path(city_root, building.addr());
    if rules.exists() {
        out.push(rules);
    }
    Ok(out)
}

fn write_new(path: &Path, text: &str) -> Result<(), AxError> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut handle) => std::io::Write::write_all(&mut handle, text.as_bytes())
            .map_err(|err| storage(path, &err)),

        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(storage(path, &err)),
    }
}

pub(crate) fn storage(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "lay out a building's documents",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("fix the path's permissions, then run this again")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
