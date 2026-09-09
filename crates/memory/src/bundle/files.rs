// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Bundle files: walking, counting, copying.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kernel::{EventRecord, GENESIS_PREV, Seq, chain_hash};

use crate::error::{MemoryError, io_err};
use crate::jsonl::JsonlLedger;
use crate::vfs::Vfs;

use super::manifest::{LEDGER, RESERVED};

pub(crate) fn head_of(vfs: &dyn Vfs, ledger_dir: &Path) -> Result<String, MemoryError> {
    let mut prev = GENESIS_PREV;
    let mut expected = Seq::FIRST;
    for line in read_lines(vfs, ledger_dir)? {
        let record = EventRecord::parse_line(&line).map_err(|err| MemoryError::Bundle {
            op: "verify",
            detail: err.to_string(),
        })?;
        if record.seq() != expected || record.prev() != prev {
            return Err(MemoryError::Bundle {
                op: "verify",
                detail: format!(
                    "record {} does not continue the chain",
                    record.seq().value()
                ),
            });
        }
        prev = chain_hash(&line);
        expected = expected.next().map_err(|err| MemoryError::Bundle {
            op: "verify",
            detail: err.to_string(),
        })?;
    }
    Ok(prev.to_string())
}

pub(crate) fn count_records(vfs: &dyn Vfs, ledger_dir: &Path) -> Result<u64, MemoryError> {
    let lines = read_lines(vfs, ledger_dir)?;
    u64::try_from(lines.len()).map_err(|_| MemoryError::Bundle {
        op: "count",
        detail: "more records than a count can hold".to_owned(),
    })
}

pub(crate) fn read_lines(vfs: &dyn Vfs, ledger_dir: &Path) -> Result<Vec<Vec<u8>>, MemoryError> {
    let mut out = Vec::new();
    for path in walk(vfs, ledger_dir) {
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
/// An explicit worklist rather than recursion: a city's depth is not
/// this module's to assume, and a stack overflow is not catchable.
pub(crate) fn walk(vfs: &dyn Vfs, root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        found.extend(vfs.list(&dir).unwrap_or_default());
        pending.extend(vfs.list_dirs(&dir).unwrap_or_default());
    }
    found.sort();
    found
}

/// Copies every file under `from` into `to`, keeping relative paths.
pub(crate) fn copy_tree(vfs: &mut dyn Vfs, from: &Path, to: &Path) -> Result<u64, MemoryError> {
    let mut copied = 0u64;
    let files = walk(vfs, from);
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
        if let Some(parent) = target.parent() {
            vfs.create_dir_all(parent)
                .map_err(io_err("make a bundle directory", parent))?;
        }
        let bytes = vfs
            .read(&path)
            .map_err(io_err("read a bundle file", &path))?;
        write_file(vfs, &target, &bytes)?;
        copied = copied.saturating_add(1);
    }
    Ok(copied)
}

/// The city's own files: everything outside the reserved prefix.
pub(crate) fn copy_city_files(
    vfs: &mut dyn Vfs,
    city_root: &Path,
    to: &Path,
) -> Result<u64, MemoryError> {
    let mut copied = 0u64;
    let mut seen = BTreeMap::new();
    for path in walk(vfs, city_root) {
        let Ok(relative) = path.strip_prefix(city_root) else {
            continue;
        };
        if relative.starts_with(RESERVED) {
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
        if let Some(parent) = target.parent() {
            vfs.create_dir_all(parent)
                .map_err(io_err("make a bundle directory", parent))?;
        }
        let bytes = vfs
            .read(&path)
            .map_err(io_err("read a bundle file", &path))?;
        write_file(vfs, &target, &bytes)?;
        copied = copied.saturating_add(1);
    }
    Ok(copied)
}

pub(crate) fn count_files(vfs: &dyn Vfs, root: &Path) -> Result<u64, MemoryError> {
    let mut count = 0u64;
    for path in walk(vfs, root) {
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        if relative.starts_with(RESERVED) {
            continue;
        }
        count = count.saturating_add(1);
    }
    Ok(count)
}

pub(crate) fn write_file(vfs: &mut dyn Vfs, path: &Path, bytes: &[u8]) -> Result<(), MemoryError> {
    if vfs.exists(path) {
        vfs.remove_file(path)
            .map_err(io_err("replace a bundle file", path))?;
    }
    vfs.append(path, bytes)
        .map_err(io_err("write a bundle file", path))?;
    vfs.sync_data(path)
        .map_err(io_err("flush a bundle file", path))
}

/// Opens the restored ledger, so the city is one a writer can continue.
///
/// # Errors
/// Whatever opening reports; a restored city that cannot be opened is
/// not restored.
pub fn open_restored(city_root: &Path, now: kernel::TimeMs) -> Result<PathBuf, MemoryError> {
    let dir = city_root.join(RESERVED).join(LEDGER);
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

    #[test]
    fn nothing_under_the_reserved_prefix_travels_as_a_city_file() {
        let home = tempfile::tempdir().unwrap();
        city_with(1, home.path());
        // A projection is disposable; carrying one would be a second
        // statement of what happened.
        let views = home.path().join(RESERVED).join("projection");
        std::fs::create_dir_all(&views).unwrap();
        std::fs::write(views.join("cold.redb"), b"derived").unwrap();

        let carried = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), carried.path()).unwrap();
        assert!(!carried.path().join(CITY).join(RESERVED).exists());
        assert!(!carried.path().join(CITY).join("projection").exists());
    }
}
