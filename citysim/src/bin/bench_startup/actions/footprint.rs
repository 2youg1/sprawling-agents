// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one sample wrote under a directory: how many files, how many
//! ledger lines, where one named file sits, and how to level it again
//! between samples (citysim-SPEC.md section 8-5).
//!
//! Shape: adapter. One walk of the tree answers every question here, so
//! the counts and the lookup cannot disagree about which directories were
//! entered, and a directory the walk could not read is reported once
//! rather than erased per reader.

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

/// Every regular file under `root`, in the order the walk reaches it.
///
/// Directories are entered, never returned: a directory a sample created
/// is a name on the way to a file, and counting it as something the
/// sample wrote would report the tree's shape as its weight.
fn files(root: &Path) -> Result<Vec<PathBuf>, AxError> {
    let mut found = Vec::new();
    let mut frontier = vec![root.to_path_buf()];
    while let Some(dir) = frontier.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|err| listing_failed(&dir, err))? {
            let entry = entry.map_err(|err| listing_failed(&dir, err))?;
            let kind = entry.file_type().map_err(|err| listing_failed(&dir, err))?;
            if kind.is_dir() {
                frontier.push(entry.path());
            } else {
                found.push(entry.path());
            }
        }
    }
    Ok(found)
}

/// How many files sit under `root`, directories not counted.
pub(super) fn files_under(root: &Path) -> Result<u64, AxError> {
    let count = files(root)?.len();
    u64::try_from(count).map_err(|_| too_many("files"))
}

/// The first file under `root` named `name`, in the order the walk
/// reaches it: the same lookup `install.sh` performs with `find`.
///
/// `None` is an answer rather than a failure - an archive may legitimately
/// hold no executable - and the caller that needs one says what its
/// absence means.
pub(super) fn find_named(root: &Path, name: &str) -> Result<Option<PathBuf>, AxError> {
    Ok(files(root)?.into_iter().find(|path| {
        path.file_name()
            .is_some_and(|found| found.to_string_lossy() == name)
    }))
}

/// How many lines the ledger under `dir` holds: one per append, one per
/// durability barrier.
pub(super) fn ledger_lines(dir: &Path) -> Result<u64, AxError> {
    let mut seen = 0_u64;
    for path in files(dir)? {
        let bytes = std::fs::read(&path).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the ledger",
                format!("{}: {err}", path.display()),
            )
            .with_recovery("the city this run raised should be readable")
        })?;
        for byte in bytes {
            if byte == b'\n' {
                seen = seen.saturating_add(1);
            }
        }
    }
    Ok(seen)
}

/// Takes one sample's directory away between samples, so every sample
/// writes a fresh name and the on-access scanner meets each one cold.
pub(super) fn clear(dir: &Path) -> Result<(), AxError> {
    std::fs::remove_dir_all(dir).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "clear the scratch directory",
            format!("{}: {err}", dir.display()),
        )
        .with_recovery("free space under the scratch directory and run again")
    })
}

fn listing_failed(dir: &Path, err: std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "list the scratch directory",
        format!("{}: {err}", dir.display()),
    )
    .with_recovery("the scratch directory should be readable for the length of the run")
}

fn too_many(what: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "count what one sample did",
        format!("more {what} than a count of this platform holds"),
    )
    .with_recovery("this is a defect in bench_startup: one sample outgrew the counter")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::{files_under, find_named, ledger_lines};

    /// Two directories and three files, one directory named after the
    /// executable the walk has to find: a count that counted directories
    /// would answer five, and a lookup that matched a directory would
    /// answer the wrong path.
    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("sprawling/inner")).unwrap();
        std::fs::write(dir.path().join("sprawling/sprawling"), b"binary").unwrap();
        std::fs::write(
            dir.path().join("sprawling/inner/ledger-0.jsonl"),
            b"a\nb\nc\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("README"), b"not a ledger\n").unwrap();
        dir
    }

    #[test]
    fn a_count_is_of_files_and_not_of_directories() {
        let dir = tree();
        assert_eq!(files_under(dir.path()).unwrap(), 3);
    }

    #[test]
    fn a_name_is_found_at_any_depth_and_never_as_a_directory() {
        let dir = tree();
        assert_eq!(
            find_named(dir.path(), "sprawling").unwrap(),
            Some(dir.path().join("sprawling/sprawling"))
        );
        assert_eq!(find_named(dir.path(), "absent").unwrap(), None);
    }

    #[test]
    fn a_ledger_is_counted_by_its_lines_wherever_it_was_written() {
        let dir = tree();
        assert_eq!(
            ledger_lines(&dir.path().join("sprawling/inner")).unwrap(),
            3
        );
    }
}
