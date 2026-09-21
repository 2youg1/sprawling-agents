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
//! The `!` lines are written out rather than left to the default. An
//! adopted repository may already ignore `*.md` or every dot
//! directory, and in that repository "what this building promises is in
//! history" would silently be false.
//!
//! What the block admits from the reserved subtree is named file by
//! file. A single `!.sprawling/` re-admitted every reserved subtree at
//! every depth below the building, and those subtrees are where a city
//! keeps its ledger, its object store and its references into the
//! vault: a rule written to carry five promises into history carried
//! the machine's own records with them.
//!
//! Nothing here removes a line. A directory being adopted usually
//! carries its own rules, and those bytes are exactly what adoption
//! promises not to touch.

use std::path::Path;

use kernel::layout::{BUILDING_SHELF, CONFIG_FILE, FILTERS_FILE};
use kernel::{AxCode, AxError, RESERVED_PREFIX};

use crate::policy::{DESKTOP_SCOPE_FILE, RULES_FILE};
use crate::spine_files::{HANDOFF_FILE, MEMO_FILE, ROADMAP_FILE, SPEC_FILE};

/// The file this module writes, named once.
pub const GITIGNORE_FILE: &str = ".gitignore";

/// The block, in the order it is written. Data: editing this list is
/// editing what a city keeps.
///
/// Every name is taken from the module that writes that file, so a
/// renamed document cannot leave a rule here naming something nobody
/// writes. Order is the file's grammar rather than taste: git reads the
/// last matching line, so the reserved subtree is ignored, re-admitted
/// as a directory, emptied, and then opened for the five promises one
/// at a time.
fn block() -> Vec<String> {
    vec![
        "# sprawling: what this building promises is history; what one session".to_owned(),
        "# was thinking is not.".to_owned(),
        ROADMAP_FILE.to_owned(),
        MEMO_FILE.to_owned(),
        HANDOFF_FILE.to_owned(),
        format!("!{SPEC_FILE}"),
        format!("{RESERVED_PREFIX}/"),
        format!("!/{RESERVED_PREFIX}/"),
        format!("/{RESERVED_PREFIX}/*"),
        format!("!/{RESERVED_PREFIX}/{RULES_FILE}"),
        format!("!/{RESERVED_PREFIX}/{CONFIG_FILE}"),
        format!("!/{RESERVED_PREFIX}/{FILTERS_FILE}"),
        format!("!/{RESERVED_PREFIX}/{DESKTOP_SCOPE_FILE}"),
        format!("!/{RESERVED_PREFIX}/{BUILDING_SHELF}/"),
    ]
}

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
    // Read and write under one hold: the file belongs to the project
    // and a person may be adding a line to it at the same moment.
    crate::document::edit(&path, |held| {
        let existing = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(storage(&path, &err)),
        };
        let mut out = existing.clone();
        let mut added = false;
        for rule in block() {
            if existing.lines().any(|line| line.trim() == rule) {
                continue;
            }
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            // The block is announced once, and only when something from
            // it is actually being added below.
            out.push_str(&rule);
            out.push('\n');
            added = true;
        }
        if !added {
            return Ok(());
        }
        held.replace(out.as_bytes())
    })
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// The reserved subtree is opened for the building's promises and
    /// for nothing else: the city's ledger and its object store sit in
    /// a subtree with the same name, and a line that re-admits the
    /// name admits them too.
    #[test]
    fn the_reserved_subtree_is_admitted_file_by_file_and_never_whole() {
        let block = block();
        assert!(
            !block.iter().any(|line| line == "!.sprawling/"),
            "the whole reserved subtree is re-admitted at every depth"
        );
        for promise in [RULES_FILE, CONFIG_FILE, FILTERS_FILE, DESKTOP_SCOPE_FILE] {
            let admitted = format!("!/{RESERVED_PREFIX}/{promise}");
            assert!(
                block.contains(&admitted),
                "{promise} is what this building promises and must survive an outer ignore rule"
            );
        }
        for machines_own in [
            kernel::layout::LEDGER_DIR,
            kernel::layout::CAS_DIR,
            kernel::layout::LIBRARY_DIR,
        ] {
            assert!(
                !block.iter().any(|line| line.contains(machines_own)),
                "{machines_own} is the city's own record and belongs to no project's history"
            );
        }
    }

    /// Placing the block twice appends nothing, so raising a building
    /// and adopting the directory it left behind read the same.
    #[test]
    fn a_second_placement_adds_no_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(GITIGNORE_FILE), "target/\n").unwrap();
        place(dir.path()).unwrap();
        let once = std::fs::read_to_string(dir.path().join(GITIGNORE_FILE)).unwrap();
        place(dir.path()).unwrap();
        let twice = std::fs::read_to_string(dir.path().join(GITIGNORE_FILE)).unwrap();
        assert_eq!(once, twice);
        assert!(once.lines().any(|line| line.trim() == "target/"));
    }
}
