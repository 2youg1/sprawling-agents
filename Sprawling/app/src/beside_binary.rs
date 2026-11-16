// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Homing {
    NothingBeside,

    Moved {
        from: PathBuf,
        to: PathBuf,
        relinked: Result<usize, AxError>,
    },

    OnAnotherVolume {
        at: PathBuf,
        home: PathBuf,
    },
}

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

fn after_rename(renamed: std::io::Result<()>, from: PathBuf, to: &Path) -> Result<Homing, AxError> {
    match renamed {
        Ok(()) => Ok(Homing::Moved {
            relinked: storage::Worktrees::relink_moved(to, &from)
                .map_err(storage::StorageError::into_ax),
            from,
            to: to.to_path_buf(),
        }),
        Err(err) if err.kind() == std::io::ErrorKind::CrossesDevices => {
            Ok(Homing::OnAnotherVolume {
                at: from,
                home: to.to_path_buf(),
            })
        }
        Err(err) => Err(fatal("move the city home", &from, &err)),
    }
}

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
