// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Bundle files: walking, counting, copying.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use kernel::ledger::chain_hash;
use kernel::{EventRecord, GENESIS_PREV, Seq};

use crate::alias::WriteTarget;
use crate::error::{StorageError, io_err};
use crate::jsonl::JsonlLedger;
use crate::vfs::Vfs;

use super::landing::{Bits, is_staging_name, land};

use super::manifest::RESERVED;

pub(crate) fn head_of(vfs: &dyn Vfs, ledger_dir: &Path) -> Result<String, StorageError> {
    let mut prev = GENESIS_PREV;
    let mut expected = Seq::FIRST;
    for line in read_lines(vfs, ledger_dir)? {
        let record = EventRecord::parse_line(&line).map_err(|err| StorageError::Bundle {
            op: "verify",
            detail: err.to_string(),
        })?;
        if record.seq() != expected || record.prev() != prev {
            return Err(StorageError::Bundle {
                op: "verify",
                detail: format!(
                    "record {} does not continue the chain",
                    record.seq().value()
                ),
            });
        }
        prev = chain_hash(&line);
        expected = expected.next().map_err(|err| StorageError::Bundle {
            op: "verify",
            detail: err.to_string(),
        })?;
    }
    Ok(prev.to_string())
}

pub(crate) fn count_records(vfs: &dyn Vfs, ledger_dir: &Path) -> Result<u64, StorageError> {
    let lines = read_lines(vfs, ledger_dir)?;
    u64::try_from(lines.len()).map_err(|_| StorageError::Bundle {
        op: "count",
        detail: "more records than a count can hold".to_owned(),
    })
}

pub(crate) fn read_lines(vfs: &dyn Vfs, ledger_dir: &Path) -> Result<Vec<Vec<u8>>, StorageError> {
    let mut out = Vec::new();
    for path in walk(vfs, ledger_dir)? {
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        let bytes = vfs
            .read(&path)
            .map_err(io_err("read a bundle file", &path))?;
        for line in bytes.split(|byte| *byte == b'\n') {
            if !line.is_empty() {
                out.push(line.to_vec());
            }
        }
    }
    Ok(out)
}

/// Every file under `root`, at any depth, in a deterministic order.
///
/// A directory that cannot be listed stops the walk with its own
/// failure: a subdirectory silently contributing zero files is how a
/// bundle comes out short and agrees with itself about it. So does an
/// alias: walking one would read through it, and a copy that followed a
/// link would carry bytes no name in the bundle accounts for
/// (`crates/storage/spec/Alias.lean` §8-25).
///
/// An explicit worklist rather than recursion: a city's depth is not
/// this module's to assume, and a stack overflow is not catchable.
pub(crate) fn walk(vfs: &dyn Vfs, root: &Path) -> Result<Vec<PathBuf>, StorageError> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Some(files) = present(vfs.list(&dir), &dir)? else {
            continue;
        };
        for file in &files {
            refuse_alias(file)?;
        }
        found.extend(files);
        if let Some(dirs) = present(vfs.list_dirs(&dir), &dir)? {
            for sub in &dirs {
                refuse_alias(sub)?;
            }
            pending.extend(dirs);
        }
    }
    found.sort();
    Ok(found)
}

/// One entry against the alias rule, refused whole.
fn refuse_alias(path: &Path) -> Result<(), StorageError> {
    match crate::alias::kind_at(path)? {
        Some(kind) => Err(crate::alias::refused("walk a store or bundle", path, kind)),
        None => Ok(()),
    }
}

/// What travels as a city file: the open tree and each scope's
/// governance, never git metadata and never the city's own stores.
///
/// Root `.sprawling` holds the two stores, and they travel through
/// faces of their own; a nested one is a building's own rules and is
/// part of the city. `.git` at any depth is protected metadata, and a
/// bundle that carried it would plant hooks on restore (`crates/kernel/spec/Address.lean`
/// §8-73). The names come from kernel's one list; nothing here
/// re-spells them. A staging file a crashed restore left is half of a
/// write, and `landing` alone knows its spelling.
fn travels(relative: &Path) -> bool {
    !relative.starts_with(RESERVED)
        && !relative.file_name().is_some_and(is_staging_name)
        && !relative.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|segment| segment.eq_ignore_ascii_case(kernel::GIT_METADATA))
        })
}

/// The restore door's half of the same rule: a bundle may carry city
/// files and nothing else. Export **selects** what travels; restore
/// **refuses** what never could - all or nothing, never a skip.
///
/// The one entry admitted without travelling is the repository a v0.0.6
/// export copied whole into `city/.git`: `history` imports its objects
/// and refs, no file of it is copied, and the count returned is how many
/// files it holds, which that export's manifest counted as city files.
///
/// # Errors
/// `StorageError::Bundle` naming the first entry that is not a city
/// file; `StorageError::Alias` from the walk.
pub(crate) fn only_city_files(vfs: &dyn Vfs, root: &Path) -> Result<u64, StorageError> {
    let mut repository = 0u64;
    for path in walk(vfs, root)? {
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        if relative
            .strip_prefix(kernel::GIT_METADATA)
            .is_ok_and(|inner| !inner.as_os_str().is_empty())
        {
            repository = repository.saturating_add(1);
            continue;
        }
        if !travels(relative) {
            return Err(StorageError::Bundle {
                op: "restore",
                detail: format!(
                    "{} is protected metadata or a staging file, which no bundle may hold",
                    path.display()
                ),
            });
        }
    }
    Ok(repository)
}

/// A directory that is not there holds nothing, which is an answer; a
/// directory that refuses to be read is not.
///
/// A city with no object store yet has no `cas/` to walk, and export
/// asks about it before anything has put an object in it.
fn present(
    listing: io::Result<Vec<PathBuf>>,
    dir: &Path,
) -> Result<Option<Vec<PathBuf>>, StorageError> {
    match listing {
        Ok(paths) => Ok(Some(paths)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io_err("walk a bundle directory", dir)(err)),
    }
}

/// Copies every file under `from` into `to`, keeping relative paths;
/// `root` is the directory the person named, which bounds the alias
/// check on every write.
pub(crate) fn copy_tree(
    vfs: &mut dyn Vfs,
    root: &Path,
    from: &Path,
    to: &Path,
) -> Result<u64, StorageError> {
    let mut copied = 0u64;
    let files = walk(vfs, from)?;
    if files.is_empty() {
        return Ok(0);
    }
    vfs.create_dir_all(to)
        .map_err(io_err("make a bundle directory", to))?;
    for path in files {
        let Ok(relative) = path.strip_prefix(from) else {
            continue;
        };
        let target = to.join(relative);
        // Cleared before any directory is made: making a parent through
        // a link would land a directory inside what the link reaches,
        // before the file write is refused (`crates/storage/spec/Alias.lean` §8-25).
        let cleared = WriteTarget::within("copy a bundle file", root, &target)?;
        if let Some(parent) = target.parent() {
            vfs.create_dir_all(parent)
                .map_err(io_err("make a bundle directory", parent))?;
        }
        let bytes = vfs
            .read(&path)
            .map_err(io_err("read a bundle file", &path))?;
        land(vfs, cleared, &bytes, Bits::Of(&path))?;
        copied = copied.saturating_add(1);
    }
    Ok(copied)
}

/// The city's own files: everything outside the reserved prefix, bounded
/// by `root` as [`copy_tree`] is.
pub(crate) fn copy_city_files(
    vfs: &mut dyn Vfs,
    root: &Path,
    city_root: &Path,
    to: &Path,
) -> Result<u64, StorageError> {
    let mut copied = 0u64;
    let mut seen = BTreeMap::new();
    for path in walk(vfs, city_root)? {
        let Ok(relative) = path.strip_prefix(city_root) else {
            continue;
        };
        if !travels(relative) {
            continue;
        }
        seen.insert(relative.to_path_buf(), path);
    }
    if seen.is_empty() {
        return Ok(0);
    }
    vfs.create_dir_all(to)
        .map_err(io_err("make a bundle directory", to))?;
    for (relative, path) in seen {
        let target = to.join(&relative);
        // Cleared before any directory is made, for the reason the
        // sibling walk gives.
        let cleared = WriteTarget::within("copy a city file", root, &target)?;
        if let Some(parent) = target.parent() {
            vfs.create_dir_all(parent)
                .map_err(io_err("make a bundle directory", parent))?;
        }
        let bytes = vfs
            .read(&path)
            .map_err(io_err("read a bundle file", &path))?;
        land(vfs, cleared, &bytes, Bits::Of(&path))?;
        copied = copied.saturating_add(1);
    }
    Ok(copied)
}

pub(crate) fn count_files(vfs: &dyn Vfs, root: &Path) -> Result<u64, StorageError> {
    let mut count = 0u64;
    for path in walk(vfs, root)? {
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        if !travels(relative) {
            continue;
        }
        count = count.saturating_add(1);
    }
    Ok(count)
}

/// Opens the restored ledger, so the city is one a writer can continue.
///
/// # Errors
/// Whatever opening reports; a restored city that cannot be opened is
/// not restored.
pub fn open_restored(city_root: &Path, now: kernel::TimeMs) -> Result<PathBuf, StorageError> {
    let dir = kernel::layout::CityLayout::new(city_root).ledger();
    let (_ledger, _report) = JsonlLedger::open(&dir, now)?;
    Ok(dir)
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
    use super::super::export::Bundle;
    use super::super::fixture::city_with;
    use super::super::manifest::CITY;
    use super::*;

    /// The permission a city file carries is part of the file: a
    /// read-only note (or, on Unix, an executable script) comes back
    /// the way it left.
    #[test]
    fn a_read_only_city_file_comes_back_read_only() {
        let home = tempfile::tempdir().unwrap();
        city_with(1, home.path());
        let kept = home.path().join("kept.md");
        std::fs::write(&kept, b"do not touch").unwrap();
        let mut bits = std::fs::metadata(&kept).unwrap().permissions();
        bits.set_readonly(true);
        std::fs::set_permissions(&kept, bits).unwrap();

        let carried = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), carried.path()).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        Bundle::restore(carried.path(), elsewhere.path()).unwrap();
        let read_only = |at: &Path| std::fs::metadata(at).unwrap().permissions().readonly();
        assert_eq!(
            (
                read_only(&carried.path().join(CITY).join("kept.md")),
                read_only(&elsewhere.path().join("kept.md")),
            ),
            (true, true)
        );
    }

    /// A read-only city file already in the bundle is replaced like any
    /// other, so an export can be repeated over the bundle it made.
    #[test]
    fn an_export_repeats_over_a_bundle_holding_a_read_only_file() {
        let home = tempfile::tempdir().unwrap();
        city_with(1, home.path());
        let kept = home.path().join("kept.md");
        std::fs::write(&kept, b"do not touch").unwrap();
        let mut bits = std::fs::metadata(&kept).unwrap().permissions();
        bits.set_readonly(true);
        std::fs::set_permissions(&kept, bits).unwrap();

        let carried = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), carried.path()).unwrap();
        let again = Bundle::export(home.path(), carried.path())
            .map(|_| ())
            .map_err(|err| err.to_string());
        assert_eq!(again, Ok(()));
    }

    /// The city root is the person's choice, so a link above it is
    /// where they keep the city rather than a write a run redirected.
    #[test]
    fn a_city_kept_under_a_link_exports_and_restores() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        let via = tmp.path().join("via");
        if !crate::alias::tests::place_link(false, &real, &via) {
            return;
        }
        let (city, carried, elsewhere) = (via.join("city"), via.join("bk"), via.join("back"));
        std::fs::create_dir_all(&city).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        city_with(1, &city);
        let outcome = Bundle::export(&city, &carried)
            .and_then(|_| Bundle::restore(&carried, &elsewhere))
            .map(|_| ())
            .map_err(|err| err.to_string());
        assert_eq!(outcome, Ok(()));
    }

    #[test]
    fn nothing_under_the_reserved_prefix_travels_as_a_city_file() {
        let home = tempfile::tempdir().unwrap();
        city_with(1, home.path());
        // A side index is disposable; carrying one would be a second
        // statement of what happened.
        let cache = home.path().join(RESERVED).join("index");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("ledger.idx"), b"derived").unwrap();

        let carried = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), carried.path()).unwrap();
        assert!(!carried.path().join(CITY).join(RESERVED).exists());
        assert!(!carried.path().join(CITY).join("index").exists());
    }

    /// A restore cut short leaves its staging file under the city root;
    /// the next export must not carry it as the person's file.
    #[test]
    fn a_staging_file_a_crash_left_does_not_travel() {
        let home = tempfile::tempdir().unwrap();
        city_with(1, home.path());
        std::fs::write(home.path().join("kept.md"), b"whole").unwrap();
        std::fs::write(home.path().join(".kept.md.part"), b"half").unwrap();

        let carried = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), carried.path()).unwrap();
        let city = carried.path().join(CITY);
        assert_eq!(
            (
                city.join("kept.md").exists(),
                city.join(".kept.md.part").exists()
            ),
            (true, false)
        );
    }
}
