// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The package walk by directory handles (`crates/city/spec/Library/Install.lean` §8-28).
//!
//! The root is opened from its parent without following a link, and
//! every item beneath is opened by name relative to the directory handle
//! that listed it, again without following a link, so every byte read
//! comes through a chain of handles from the root. A directory swapped
//! for a link after its judgement is outside anything the walk can open:
//! the name either fails to open or opens the link itself, and the
//! handle's own metadata refuses it. A listing only contributes names,
//! so even a listing a swap could fool - Windows lists by the path read
//! back from the handle - hands the walk names it opens relative to the
//! true handle.

use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use kernel::{AxCode, AxError};

use super::{not_a_source, plain, refuses_link, source_io};
use crate::library::reading;

/// The most file bytes one package may carry. The precheck holds every
/// byte of a package while it judges it and the store keeps the package
/// as one blob, so a skill - text and the small scripts beside it - is
/// bounded well above its kind and well below what a person's machine
/// can hold twice.
pub(in crate::library::install) const PACKAGE_BYTES_LIMIT: u64 = 32 * 1024 * 1024;

/// One item the walk met: its path relative to the package with
/// segments joined by `/`, and a file's bytes (`None` for a directory).
pub(super) struct Found {
    pub(super) path: String,
    pub(super) bytes: Option<Vec<u8>>,
}

/// Every item under the package at `dir`, in the order the walk met it.
///
/// # Errors
/// `E_INVALID_ARGS` for a link, an item that is neither a directory nor
/// a file, and a package past [`PACKAGE_BYTES_LIMIT`]; the precheck's
/// storage refusals for what could not be listed, opened or read.
pub(super) fn items(dir: &Path) -> Result<Vec<Found>, AxError> {
    let mut found = Vec::new();
    let mut budget = PACKAGE_BYTES_LIMIT;
    let mut open = vec![(open_root(dir)?, String::new())];
    while let Some((at, prefix)) = open.pop() {
        let listed = at
            .entries()
            .map_err(|err| source_io(&dir.join(&prefix), &err))?;
        for entry in listed {
            let entry = entry.map_err(|err| source_io(&dir.join(&prefix), &err))?;
            let shown = dir.join(&prefix).join(entry.file_name());
            let path = format!("{prefix}{}", reading::spelled(&shown)?);
            let kind = entry.file_type().map_err(|err| source_io(&shown, &err))?;
            if kind.is_symlink() {
                return Err(refuses_link(&shown));
            } else if kind.is_dir() {
                let opened = open_directory(&at, &entry.file_name(), &shown)?;
                open.push((opened, format!("{path}/")));
                found.push(Found { path, bytes: None });
            } else if kind.is_file() {
                let bytes = read_file(&at, &entry.file_name(), &shown, &mut budget)?;
                found.push(Found {
                    path,
                    bytes: Some(bytes),
                });
            } else {
                return Err(not_a_source(&shown, "it is neither a directory nor a file"));
            }
        }
    }
    Ok(found)
}

/// Opens the package's root from its parent, without following a link.
fn open_root(dir: &Path) -> Result<Dir, AxError> {
    let Some(leaf) = dir.file_name() else {
        return Err(not_a_source(dir, "it names no directory"));
    };
    let parent = dir
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = Dir::open_ambient_dir(parent, cap_std::ambient_authority())
        .map_err(|err| source_io(dir, &err))?;
    open_directory(&parent, leaf, dir)
}

/// Opens one directory by name relative to the handle that listed it,
/// and refuses when what the handle holds is not a plain directory.
fn open_directory(at: &Dir, name: &OsStr, shown: &Path) -> Result<Dir, AxError> {
    let handle = at
        .open_dir_nofollow(name)
        .map_err(|err| source_io(shown, &err))?
        .into_std_file();
    let meta = handle.metadata().map_err(|err| source_io(shown, &err))?;
    if !(meta.is_dir() && plain(&meta)) {
        return Err(refuses_link(shown));
    }
    Ok(Dir::from_std_file(handle))
}

/// Reads one file by name relative to the handle that listed it,
/// without following a link, and refuses what is not a plain file or
/// what would carry the package past [`PACKAGE_BYTES_LIMIT`]: by the
/// length the handle reports before a byte is held, and by the bytes
/// read when the file grew under the read.
fn read_file(at: &Dir, name: &OsStr, shown: &Path, budget: &mut u64) -> Result<Vec<u8>, AxError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = at
        .open_with(name, &options)
        .map_err(|err| source_io(shown, &err))?
        .into_std();
    let meta = file.metadata().map_err(|err| source_io(shown, &err))?;
    if !(meta.is_file() && plain(&meta)) {
        return Err(refuses_link(shown));
    }
    if meta.len() > *budget {
        return Err(too_large(shown));
    }
    let mut bytes = Vec::new();
    file.take(budget.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|err| source_io(shown, &err))?;
    let read = u64::try_from(bytes.len()).map_err(|_| too_large(shown))?;
    *budget = budget.checked_sub(read).ok_or_else(|| too_large(shown))?;
    Ok(bytes)
}

fn too_large(path: &Path) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!(
            "{} carries the package past {PACKAGE_BYTES_LIMIT} bytes",
            path.display()
        ),
    )
    .with_recovery(
        "move large files out of the package, keeping the text and small scripts a skill \
         reads, then install again",
    )
}
