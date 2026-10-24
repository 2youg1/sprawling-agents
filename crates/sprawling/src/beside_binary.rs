// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A city an earlier release kept beside the binary, and its one move
//! to where this release keeps cities (`crates/sprawling/spec/Firstrun.lean`
//! §8-8).
//!
//! Releases up to 0.0.12 put a city nobody named in `<binary's
//! folder>/city`. An upgrade unpacks into another folder, so that city
//! is left behind; this module moves it once, as one folder, and points
//! the worktrees git recorded by absolute path at the new place.

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

/// What [`bring_home`] found and did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Homing {
    /// There is no city beside the binary.
    NothingBeside,
    /// The city at `from` is now at `to`. `relinked` is how many
    /// worktrees were pointed at it, or why their links could not be
    /// rewritten: the move stands either way, because a tree whose links
    /// git cannot follow is taken back and checked out again the next
    /// time the city places one (`crates/sprawling/spec/Supervising.lean`).
    Moved {
        from: PathBuf,
        to: PathBuf,
        relinked: Result<usize, AxError>,
    },
}

/// Moves the city beside the binary in `exe_dir` to `to`, when there is
/// one.
///
/// The move is one rename, so a reader finds the city whole in one
/// place or the other. The writer lock is taken first to learn that no
/// city is running from the old folder. Outside Windows it is held
/// through the rename, so no city can start from the old folder in
/// between. Windows refuses to rename a folder with a file open inside,
/// the lock file included, so there it is let go first; that same
/// refusal turns a city that starts in between into a failed rename
/// rather than a city moved under a running process.
///
/// # Errors
/// `E_LEDGER_HELD` when a city is running from the old folder;
/// `E_INVALID_ARGS` when `to` already holds a city or other files, or
/// lies on another volume (a copy across volumes is not attempted);
/// `E_STORAGE_FATAL` when a folder
/// cannot be read, made or renamed. A worktree link that cannot be
/// rewritten after the rename is answered in [`Homing::Moved`], not here.
pub fn bring_home(exe_dir: &Path, to: &Path) -> Result<Homing, AxError> {
    let from = exe_dir.join(accounting::home::CITY_DIR);
    match city::has_history(&from)? {
        city::History::Absent => return Ok(Homing::NothingBeside),
        city::History::Present => {}
    }
    vacate(to)?;
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| fatal("make the folder cities live in", parent, &err))?;
    }
    let ledger = kernel::layout::CityLayout::new(&from).ledger();
    let held = storage::LedgerHold::take(&ledger).map_err(storage::StorageError::into_ax)?;
    #[cfg(windows)]
    drop(held);
    let renamed = std::fs::rename(&from, to);
    #[cfg(not(windows))]
    drop(held);
    after_rename(renamed, from, to)
}

/// What the rename's outcome means for the city that was at `from`.
fn after_rename(renamed: std::io::Result<()>, from: PathBuf, to: &Path) -> Result<Homing, AxError> {
    renamed.map_err(|err| {
        if err.kind() == std::io::ErrorKind::CrossesDevices {
            refused(
                to,
                "is on another volume than the city beside the binary",
                format!(
                    "move the folder {} there yourself, or write city = {} in the person's config.toml",
                    from.display(),
                    toml::Value::String(from.display().to_string())
                ),
            )
        } else {
            fatal("move the city home", &from, &err)
        }
    })?;
    let relinked =
        storage::Worktrees::relink_moved(to, &from).map_err(storage::StorageError::into_ax);
    Ok(Homing::Moved {
        from,
        to: to.to_path_buf(),
        relinked,
    })
}

/// Makes sure `to` can receive a city: absent, or an empty folder,
/// which is removed so the rename can put the city in its place.
fn vacate(to: &Path) -> Result<(), AxError> {
    match city::has_history(to)? {
        city::History::Present => {
            return Err(refused(
                to,
                "already holds a city, and another one is beside the binary",
                "write city = \"<the one you want>\" in the person's config.toml".to_owned(),
            ));
        }
        city::History::Absent => {}
    }
    match std::fs::read_dir(to) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(fatal("read the folder the city moves to", to, &err)),
        Ok(mut entries) => match entries.next() {
            None => std::fs::remove_dir(to)
                .map_err(|err| fatal("clear the folder the city moves to", to, &err)),
            Some(_) => Err(refused(
                to,
                "holds files, so the city beside the binary cannot move there",
                "empty or rename that folder, or write city = \"<path>\" in the person's config.toml"
                    .to_owned(),
            )),
        },
    }
}

fn refused(to: &Path, why: &str, recovery: String) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "move the city beside the binary home",
        format!("{} {why}", to.display()),
    )
    .with_recovery(recovery)
}

fn fatal(action: &str, path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        action.to_owned(),
        format!("{}: {err}", path.display()),
    )
    .with_recovery("fix that folder's permissions or free disk space, then start again")
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
