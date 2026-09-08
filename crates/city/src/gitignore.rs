// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a building promises is tracked; what one session was thinking
//! is not (city-SPEC.md section 8-21).
//!
//! `SPEC.md` and the building's own reserved subtree go into history,
//! because a reader who clones this repository a year from now needs
//! the promises and the rules they were made under. `Roadmap.md`,
//! `Memo.md` and `Handoff.md` do not: they are where one session keeps
//! what it is currently thinking, and a plan that is rewritten every
//! hour turns a history into a diff of nobody's decisions.
//!
//! The two `!` lines are written out rather than left to the default.
//! An adopted repository may already ignore `*.md` or every dot
//! directory, and in that repository "what this building promises is in
//! history" would silently be false.
//!
//! Nothing here removes a line. A directory being adopted usually
//! carries its own rules, and those bytes are exactly what adoption
//! promises not to touch.

use std::path::Path;

use kernel::{AxCode, AxError};

/// The file this module writes, named once.
pub const GITIGNORE_FILE: &str = ".gitignore";

/// The block, in the order it is written. Data: editing this list is
/// editing what a city keeps.
const BLOCK: &[&str] = &[
    "# sprawling: what this building promises is history; what one session",
    "# was thinking is not.",
    "Roadmap.md",
    "Memo.md",
    "Handoff.md",
    "!SPEC.md",
    "!.sprawling/",
];

/// Adds this city's rules to a building's `.gitignore`, keeping every
/// line that is already there.
///
/// Idempotent by line: a rule already present in the file is not added
/// again, so raising a building twice, or adopting a directory the city
/// once raised, appends nothing. The comparison is on the trimmed line,
/// which is how a person reading the file compares them.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path when the file cannot be read or
/// written. A file that is not there is not a failure — it is the
/// ordinary case for a building the city just raised.
pub(crate) fn place(building_root: &Path) -> Result<(), AxError> {
    let path = building_root.join(GITIGNORE_FILE);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(storage(&path, &err)),
    };
    let mut out = existing.clone();
    let mut added = false;
    for rule in BLOCK {
        if existing.lines().any(|line| line.trim() == *rule) {
            continue;
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        // The block is announced once, and only when something from it
        // is actually being added below.
        out.push_str(rule);
        out.push('\n');
        added = true;
    }
    if !added {
        return Ok(());
    }
    std::fs::write(&path, out.as_bytes()).map_err(|err| storage(&path, &err))
}

/// A room holds one session's workplace, and the whole of it stays on
/// this machine. One line rather than a pattern in the building's own
/// file: a room is a subdirectory a person named on the spot, so at the
/// moment a building is raised there is nothing yet to name, and a
/// wildcard guessing which subdirectories are rooms would catch the
/// source tree of a directory the city adopted.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path when the file cannot be written.
pub(crate) fn seal_room(room: &Path) -> Result<(), AxError> {
    let path = room.join(GITIGNORE_FILE);
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut handle) => {
            std::io::Write::write_all(&mut handle, b"*\n").map_err(|err| storage(&path, &err))
        }
        // Whatever the room already says about itself is the room's.
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(storage(&path, &err)),
    }
}

fn storage(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "write a building's ignore rules",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("fix the path's permissions, then run this again")
}
