// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every bundle write, and the one way it reaches disk (memory-SPEC 8-12).
//!
//! The bytes go to a staging file beside the target, are flushed, take
//! the original permissions, and are renamed over the name. A reader
//! finds the old file or the whole new one, never a name removed before
//! its replacement was on the device, and the rename replaces the
//! directory entry, so a hard link at the name is broken rather than
//! written through.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use crate::alias::WriteTarget;
use crate::error::{MemoryError, io_err};
use crate::vfs::Vfs;

/// Where a landed file's permissions come from.
pub(crate) enum Bits<'a> {
    /// A copy: the source file's permissions travel with its bytes.
    Of(&'a Path),
    /// A file the bundle writes itself: the permissions of the file it
    /// replaces, and the creation default when it replaces nothing.
    OfReplaced,
}

/// Lands `bytes` at `target`, consuming the cleared target so no write
/// can reach an uncleared name.
///
/// Permissions are copied after the flush, because once a read-only bit
/// is on the staging file Windows refuses the write handle a flush
/// opens.
///
/// # Errors
/// `MemoryError::Io` naming the staging file or the target for a write,
/// flush, permission copy or rename the filesystem refuses; the target
/// keeps its previous content in every one of those cases.
pub(crate) fn land(
    vfs: &mut dyn Vfs,
    target: WriteTarget,
    bytes: &[u8],
    bits: Bits<'_>,
) -> Result<(), MemoryError> {
    let path = target.as_path();
    let staged = staging_path(path)?;
    if vfs.exists(&staged) {
        vfs.remove_file(&staged)
            .map_err(io_err("clear a stale bundle staging file", &staged))?;
    }
    vfs.append(&staged, bytes)
        .map_err(io_err("stage a bundle file", &staged))?;
    vfs.sync_data(&staged)
        .map_err(io_err("flush a bundle file", &staged))?;
    let original = match bits {
        Bits::Of(source) => Some(source),
        Bits::OfReplaced => vfs.exists(path).then_some(path),
    };
    if let Some(original) = original {
        vfs.copy_permissions(original, &staged)
            .map_err(io_err("keep a bundle file's permissions", original))?;
    }
    vfs.rename(&staged, path)
        .map_err(io_err("replace a bundle file", path))?;
    match path.parent() {
        Some(dir) => vfs
            .sync_dir(dir)
            .map_err(io_err("record a bundle file's name", dir)),
        None => Ok(()),
    }
}

/// `.<name>.part` beside `path`, built from the name as the operating
/// system spells it so a name that is not valid Unicode still stages.
fn staging_path(path: &Path) -> Result<PathBuf, MemoryError> {
    let name = path.file_name().ok_or_else(|| {
        io_err("stage a bundle file", path)(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a bundle file needs a file name of its own",
        ))
    })?;
    let mut staged = OsString::from(".");
    staged.push(name);
    staged.push(".part");
    Ok(path.with_file_name(staged))
}
