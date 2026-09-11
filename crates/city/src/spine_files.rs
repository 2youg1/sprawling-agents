// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The documents a building keeps its long work in, and the job file a
//! run starts from.
//!
//! Three of the four spine documents are laid out here — `Roadmap.md`
//! and `Memo.md` with the building, `Handoff.md` with each room. The
//! fourth, `BUILDING.md`, is written where its meaning lives
//! (`crate::building`, read by `crate::policy`): one file, one writer.
//!
//! The handoff is the room's rather than the building's:
//! two sessions of one building can freeze at once, and one file for
//! both would be two contents fighting over one name.
//!
//! Nothing here overwrites. A building that has been working keeps its
//! plan, its decisions and its handoff, whatever is run against it
//! afterwards. The one file that is rewritten is `JOB.md`, because it
//! holds the task of the current session rather than a record of past
//! ones — and the record of past ones is in the ledger, where a rewrite
//! cannot reach it.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError};

use crate::policy::building_path;

pub(crate) mod hall;

mod blank;
use blank::{empty_roadmap, is_blank_form};

pub use hall::{CLERK_FILE, MAYOR_FILE, hall_identity_path, lay_out_hall_identities};

/// The plan: the single denominator for progress in a building.
///
/// Named in `kernel` beside the table grammar that reads it, and spelled
/// here so a caller of this module has one place to look. One authority,
/// one alias — not two.
pub const ROADMAP_FILE: &str = kernel::ROADMAP_FILE;
/// Decisions and corrections.
pub(crate) const MEMO_FILE: &str = "Memo.md";
/// What the next agent needs before it starts.
pub const HANDOFF_FILE: &str = "Handoff.md";
/// The task of one session, in the room it is run from.
pub const JOB_FILE: &str = "JOB.md";
/// What this building is and the decisions it holds, written before the
/// code that follows them. The one spine document that is committed:
/// a promise a clone cannot read is not a promise.
pub const SPEC_FILE: &str = "SPEC.md";
/// The city's own instructions, read into every prefix.
pub const CITY_FILE: &str = "City.md";
/// The conventions a project brings with it. **The city neither writes
/// this file nor owns it**; it is listed here because a resident is
/// given it rather than sent to fetch it. How it is matched and where
/// it lands in the prompt is `bin::assembly::freezing::building_segment`.
pub const AGENTS_FILE: &str = "AGENTS.md";

const ROADMAP_TEMPLATE: &str = include_str!("../../../docs/templates/Roadmap.md");
const MEMO_TEMPLATE: &str = include_str!("../../../docs/templates/Memo.md");
const HANDOFF_TEMPLATE: &str = include_str!("../../../docs/templates/Handoff.md");
/// The condensed form of the seventeen-section crate SPEC: the same
/// section order, with the sections that only a crate in this workspace
/// owes left out. One template, so a building's SPEC and a crate's SPEC
/// stay one shape rather than two competing ones.
const SPEC_TEMPLATE: &str = include_str!("../../../docs/templates/SPEC.md");
const NAME_PLACEHOLDER: &str = "<building name>";
const PROJECT_PLACEHOLDER: &str = "<project name>";

/// What a dispatch knows about the work when the job file is written.
pub struct JobBrief<'a> {
    /// One line: what to produce.
    pub task: &'a str,
    /// What counts as success, what counts as failure, when to stop.
    pub goal: &'a str,
}

/// What one session was given to work from.
///
/// Exhaustive, and the two arms are different situations rather than a
/// present and an absent value: a session either carries out a task
/// somebody wrote down, or works with the person directly and takes the
/// work from the conversation. The prefix says which, because an agent
/// told to read a job file that nobody wrote spends its first turn
/// looking for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunBrief {
    /// The job file's own text, as it was written to the room.
    Job { text: String },
    /// Nobody assigned this session a task: the person is here.
    Principal,
}

/// What the prefix says when there is no job file.
///
/// It states the situation rather than the absence, because "there is no
/// JOB.md" describes the disk and what the agent needs is what to do
/// instead.
const PRINCIPAL_BRIEF: &str = "No task file was written for this session. You are working with the \
     person directly: what to do arrives in the conversation, and you \
     answer there.\n";

impl RunBrief {
    /// The bytes this brief contributes to the prefix's run segment.
    #[must_use]
    pub fn segment_text(&self) -> &str {
        match self {
            RunBrief::Job { text } => text,
            RunBrief::Principal => PRINCIPAL_BRIEF,
        }
    }
}

/// Lays down this session's brief and answers which of the two it is.
///
/// A stated goal is what makes a task a job: the file's one irreplaceable
/// section says when to stop, and a form with that field blank teaches an
/// agent that stopping is undefined. So a dispatch that states a goal
/// gets a job file, and one that does not is a conversation.
///
/// **The brief is this dispatch's, never what an earlier session left in
/// the room.** A job file from last week is still on disk and can still
/// be read; what it may not do is present itself as the task of a session
/// nobody assigned one to.
///
/// # Errors
/// Propagates a room that cannot be created or written.
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

/// Where a room's handoff lives.
///
/// The one place this path is spelled, for the same reason `job_path`
/// and `roadmap_path` are.
#[must_use]
pub fn handoff_path(city_root: &Path, room: &Address) -> PathBuf {
    let mut path = city_root.to_path_buf();
    for segment in room.as_str().split('/') {
        path.push(segment);
    }
    path.push(HANDOFF_FILE);
    path
}

/// Lays the blank handoff form down in a room that was just opened.
///
/// # Errors
/// Propagates a room that cannot be written.
pub(crate) fn lay_out_handoff(room_dir: &Path, room: &Address) -> Result<(), AxError> {
    write_new(
        &room_dir.join(HANDOFF_FILE),
        &HANDOFF_TEMPLATE.replace(NAME_PLACEHOLDER, room.as_str()),
    )
}

/// What the last run in this room left for the next one.
///
/// `None` says one thing only: there is nothing here worth carrying -
/// either no file, or a form still holding the template's own
/// parenthetical guidance, which in the prefix costs the same bytes as a
/// filled one and carries nothing. A file that exists and cannot be read
/// is a third fact and is reported, because a prefix that quietly omits
/// it tells the next session there was no handoff.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path, for every failure except a file
/// that is not there.
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

/// Lays out the spine documents a building starts with.
///
/// # Errors
/// Propagates a directory that cannot be created or written.
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

/// Where the job file of a run at `addr` lives.
#[must_use]
pub fn job_path(city_root: &Path, addr: &Address) -> PathBuf {
    let mut path = city_root.to_path_buf();
    for segment in addr.as_str().split('/') {
        path.push(segment);
    }
    path.push(JOB_FILE);
    path
}

/// Where a building's plan lives.
///
/// The one place this path is spelled. A caller that joins
/// `city_root/<addr>/Roadmap.md` for itself becomes a second authority
/// for where the plan is, and it keeps working after the real one moves.
#[must_use]
pub fn roadmap_path(city_root: &Path, building_addr: &Address) -> PathBuf {
    let mut path = city_root.to_path_buf();
    for segment in building_addr.as_str().split('/') {
        path.push(segment);
    }
    path.push(ROADMAP_FILE);
    path
}

/// A building's plan as it stands, or an empty document when the
/// building has not been given one yet.
///
/// "Not laid out yet" and "could not be read" are different facts, and
/// only the first one means an empty plan. Everything else - a directory
/// where the file belongs, a permission this process does not have, a
/// device that stopped answering - is reported, because a caller that
/// reads those as an empty plan goes on to tell somebody their claim
/// lost a race that never ran.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path, for every failure except a file
/// that is not there.
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

/// Writes the job file for one run and returns the bytes written, so the
/// caller can record the same text as history without reading the file
/// back and hoping it is unchanged.
///
/// # Errors
/// Propagates a room that cannot be created or written.
pub fn write_job(
    city_root: &Path,
    addr: &Address,
    brief: &JobBrief<'_>,
) -> Result<String, AxError> {
    let path = job_path(city_root, addr);
    if let Some(room) = path.parent() {
        std::fs::create_dir_all(room).map_err(|err| storage(room, &err))?;
    }
    let text = format!(
        "# {JOB_FILE} — {}\n\n> The task for this session. Read it in full and leave it \
         unchanged.\n\n## Task\n\n{}\n\n## Goal\n\n{}\n",
        brief.task, brief.task, brief.goal
    );
    std::fs::write(&path, text.as_bytes()).map_err(|err| storage(&path, &err))?;
    Ok(text)
}

/// The norm documents a run at `addr` is aligned by, in reading order:
/// how the city works, then how this building works.
///
/// The progress documents (`Roadmap.md`, `Memo.md`, the last frozen
/// point) are not here — they are what a run reads to know where the
/// work stands, and they are named per run, not per address.
///
/// # Errors
/// Propagates the reserved-subtree refusal: an address with no building
/// has no building norms.
pub fn norms(city_root: &Path, addr: &Address) -> Result<Vec<PathBuf>, AxError> {
    let building = crate::building::Building::of(addr)?;
    let mut out = vec![city_root.join(CITY_FILE)];
    let rules = building_path(city_root, building.addr());
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
        // A document that is already there is the building's own work.
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
