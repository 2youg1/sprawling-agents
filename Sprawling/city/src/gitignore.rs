// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::{
    ARCHIVE_DIR, BUILDING_SHELF, CONFIG_FILE, FILTERS_FILE, JOB_FILE, RUN_ID_PATTERN,
    TRANSCRIPT_EXT, URBANITE_FILE,
};
use kernel::{AxCode, AxError, RESERVED_PREFIX};

use crate::policy::{DESKTOP_SCOPE_FILE, RULES_FILE};
use crate::spine_files::{HANDOFF_FILE, MEMO_FILE, ROADMAP_FILE, SPEC_FILE};

pub const GITIGNORE_FILE: &str = ".gitignore";

fn block() -> Vec<String> {
    vec![
        "# sprawling: what this building promises is history; what one session".to_owned(),
        "# was thinking is not.".to_owned(),
        ROADMAP_FILE.to_owned(),
        MEMO_FILE.to_owned(),
        HANDOFF_FILE.to_owned(),
        JOB_FILE.to_owned(),
        URBANITE_FILE.to_owned(),
        format!("{RUN_ID_PATTERN}.{TRANSCRIPT_EXT}"),
        format!("/{ARCHIVE_DIR}/"),
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

fn city_block() -> Vec<String> {
    vec![
        "# sprawling: this city's own records stay out of the workspace's history.".to_owned(),
        format!("/{RESERVED_PREFIX}/"),
    ]
}

pub(crate) fn place(building_root: &Path) -> Result<(), AxError> {
    append_missing(&building_root.join(GITIGNORE_FILE), &block())
}

pub fn place_city(city_root: &Path) -> Result<(), AxError> {
    append_missing(&city_root.join(GITIGNORE_FILE), &city_block())
}

pub fn place_everywhere(city_root: &Path) -> Result<(), AxError> {
    place_city(city_root)?;
    for addr in crate::building::all(city_root)? {
        let building = crate::building::Building::of(&addr)?;
        place(&building.root(city_root))?;
    }
    Ok(())
}

fn append_missing(path: &Path, rules: &[String]) -> Result<(), AxError> {
    crate::document::edit(path, |held| {
        let existing = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(storage(path, &err)),
        };
        let missing: Vec<&String> = rules
            .iter()
            .filter(|rule| !existing.lines().any(|line| line.trim() == rule.as_str()))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let mut out = existing.clone();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        for rule in missing {
            out.push_str(rule);
            out.push('\n');
        }
        held.replace(out.as_bytes())
    })
}

pub(crate) fn seal_room(room: &Path) -> Result<(), AxError> {
    crate::document::replace(&room.join(GITIGNORE_FILE), b"*\n")
}

fn storage(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "write the city's ignore rules",
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
