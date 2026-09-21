// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Putting a document on disk whole, or leaving the old one there
//! (city-SPEC.md section 8-27).
//!
//! Every file the city writes is read back by something that parses it:
//! a configuration layer, a building's rules, a job brief. A write that
//! truncates the target and then streams bytes into it has a window in
//! which the file on disk is neither the old document nor the new one,
//! and a reader that arrives inside that window is refused by a parser
//! rather than served by either version. The window closes at a power
//! cut and stays open, so the next run of that room, that building, or
//! that whole city fails until somebody edits the file by hand.
//!
//! **The one writer of a document that is parsed back, inside a city or
//! not.** The person's own `<home>/.sprawling/config.toml` is read back
//! by a parser exactly as a `CONFIG.toml` is, and it is written by the
//! same command traffic, so it is held and replaced through this door
//! rather than through a second implementation of whole-or-nothing
//! writing that would be a second authority for the property below.
//! That is why this module is reachable from outside this crate while
//! everything it writes for the city itself is not.
//!
//! Two properties close it, and this module is the only place either is
//! spelled:
//!
//! - **Whole or not at all.** The bytes go to a staging file beside the
//!   target, that file is flushed to the device, and only then does a
//!   rename put it in place. A rename over an existing path is one
//!   operation on every filesystem this runs on, so a reader sees the
//!   old document or the new one.
//! - **One writer at a time.** A read-modify-write of one document is
//!   held against every other writer of that same document in this
//!   process, so two sessions editing one `CONFIG.toml` cannot each
//!   read the same original and write back over the other's change.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use kernel::{AxCode, AxError};

/// What a staging file is called: the target's name, hidden by a dot
/// and suffixed, in the target's own directory.
///
/// The same name every time rather than a unique one: a writer killed
/// between the flush and the rename leaves this file behind, and a
/// fixed name means the next write of that document reuses it instead
/// of growing a directory of debris nobody can attribute. The dot keeps
/// it out of every scan the city makes, all of which skip dot entries.
const STAGING_SUFFIX: &str = ".staging";

/// Replaces `path` with `body`, creating the directories above it.
///
/// The document a reader finds is the old one until the whole of the
/// new one is on the device. Use [`edit`] instead when the new content
/// is computed from the old.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path, for a directory that cannot be
/// created, a staging file that cannot be written or flushed, and a
/// rename the filesystem refuses.
pub(crate) fn replace(path: &Path, body: &[u8]) -> Result<(), AxError> {
    edit(path, |held| held.replace(body))
}

/// Runs `act` with `path` held against every other writer of `path` in
/// this process.
///
/// This is the door for a read-modify-write: read inside `act`, decide,
/// and call [`Held::replace`] before returning. Two callers that read
/// the same original and wrote back in turn would lose the first one's
/// change, and nothing in a file system prevents that — a lock does.
///
/// The lock is this process's. A person editing the same file in a text
/// editor is not held back by it, and cannot be without a facility no
/// dependency in this tree offers; the atomic replacement is what keeps
/// that person's editor from ever reading half a document.
///
/// # Errors
/// Propagates whatever `act` returns.
pub fn edit<T>(
    path: &Path,
    act: impl FnOnce(&Held<'_>) -> Result<T, AxError>,
) -> Result<T, AxError> {
    let slot = slot(path);
    // A poisoned lock means some other writer panicked while holding
    // it. The document itself is unharmed — it is only ever changed by
    // a rename — so the right move is to take the lock and carry on
    // rather than to refuse every later write of this file.
    let _guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    act(&Held { path })
}

/// One document, held against every other writer of it in this process.
///
/// Obtainable only from [`edit`], which is what makes "the lock is held
/// while this document is replaced" a property of the type rather than
/// a rule each call site has to remember.
pub struct Held<'a> {
    path: &'a Path,
}

impl Held<'_> {
    /// Writes `body` as the whole of this document.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` naming the path that failed: the directory, the
    /// staging file, or the target.
    pub fn replace(&self, body: &[u8]) -> Result<(), AxError> {
        let dir = self.path.parent().ok_or_else(|| {
            storage(
                self.path,
                "a document needs a directory to sit in".to_owned(),
            )
        })?;
        std::fs::create_dir_all(dir).map_err(|err| storage(dir, err.to_string()))?;
        let staged = staging_path(self.path)?;
        stage(&staged, body)?;
        // On every filesystem this runs on, a rename over an existing
        // path is one operation: a reader holds the old document or the
        // new one and there is no third answer.
        std::fs::rename(&staged, self.path).map_err(|err| {
            storage(
                self.path,
                format!("{} could not take its place: {err}", staged.display()),
            )
        })?;
        settle(dir)
    }
}

/// Writes the whole body into the staging file and flushes it to the
/// device, so the rename that follows moves bytes that survive a power
/// cut rather than a promise the page cache has not kept yet.
fn stage(staged: &Path, body: &[u8]) -> Result<(), AxError> {
    let mut file = std::fs::File::create(staged).map_err(|err| storage(staged, err.to_string()))?;
    file.write_all(body)
        .map_err(|err| storage(staged, err.to_string()))?;
    file.sync_all()
        .map_err(|err| storage(staged, err.to_string()))
}

/// Records the rename itself, not just the bytes it moved.
///
/// A directory entry lives in the directory, so flushing the file is
/// not enough: without this a crash can leave the new content on the
/// device and the old name still pointing at the old inode.
#[cfg(unix)]
fn settle(dir: &Path) -> Result<(), AxError> {
    std::fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|err| storage(dir, err.to_string()))
}

/// Windows has no handle a process can open on a directory to flush it,
/// and needs none: `MoveFileEx` with `REPLACE_EXISTING`, which is what
/// `rename` calls, is journalled by the filesystem itself.
#[cfg(not(unix))]
fn settle(_dir: &Path) -> Result<(), AxError> {
    Ok(())
}

/// The staging file beside `path`.
///
/// Built from the target's own file name as the operating system spells
/// it, so a document whose name is not valid Unicode is staged under a
/// name derived from it rather than refused.
fn staging_path(path: &Path) -> Result<PathBuf, AxError> {
    let name = path
        .file_name()
        .ok_or_else(|| storage(path, "a document needs a file name of its own".to_owned()))?;
    let mut staged = OsString::from(".");
    staged.push(name);
    staged.push(STAGING_SUFFIX);
    Ok(path.with_file_name(staged))
}

/// The lock for one document, made on first use and kept for the life
/// of the process.
///
/// One entry per document this process has written. A city has as many
/// documents as it has buildings and rooms, so the table is bounded by
/// the city rather than by how long the server has been up.
fn slot(path: &Path) -> Arc<Mutex<()>> {
    static WRITERS: OnceLock<Mutex<BTreeMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let table = WRITERS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut table = table.lock().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(
        table
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    )
}

/// One refusal shape for every way a document can fail to land, so the
/// recovery line is written once instead of at each of the city's write
/// faces.
fn storage(path: &Path, why: String) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "replace a document",
        format!("{}: {why}", path.display()),
    )
    .with_recovery("make the directory writable and check the disk has room, then save again")
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
