// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The shelves this city mounts from elsewhere on this machine
//! (`city-SPEC.md` section 8-8).
//!
//! A shelf is a directory another program keeps its skills in, and the
//! city reads it and never writes it. Which directories those are is
//! the city's own configuration, and only the city's: a shelf is
//! mounted for every building at once, so a building or a room mounting
//! one would be a scope admitting a file nobody who keeps this city
//! chose.

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError};

use super::ladder;

/// The directories this city's own configuration mounts read-only beside
/// its own shelves, in the order it lists them.
///
/// **The city's rung alone, not the ladder.** A shelf is a directory on
/// this machine, and it is mounted for every building at once; a
/// building or a room mounting one would be a scope admitting a file
/// nobody who keeps this city chose.
///
/// # Errors
/// Refuses an entry that is not a directory path this machine can open
/// (see [`shelf_path`]), and propagates a configuration file that exists
/// and cannot be read or parsed.
pub fn city_shelves(city_root: &Path, home: &Path) -> Result<Vec<PathBuf>, AxError> {
    let layer = ladder::stated(&CityLayout::new(city_root).city_config())?;
    let Some(stated) = layer.shelves() else {
        return Ok(Vec::new());
    };
    let mut shelves = Vec::with_capacity(stated.len());
    for raw in stated {
        shelves.push(shelf_path(raw, home)?);
    }
    Ok(shelves)
}

/// How one shelf in `[skills] shelves` becomes a directory.
///
/// `~` means this person's home directory, and an entry it starts is the
/// only kind that may be relative: a committed city configuration must
/// not carry one machine's absolute paths, and every other entry is a
/// path this machine can open as written.
///
/// # Errors
/// `E_CONFIG_INVALID`, naming the entry, when it is not a directory on
/// this machine: empty, `~` with no separator after it, or a relative
/// path that is not `~`-rooted.
fn shelf_path(raw: &str, home: &Path) -> Result<PathBuf, AxError> {
    let text = raw.trim();
    let named = |because: &str| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "mount an external skill shelf",
            format!("`{raw}` in `[skills] shelves` {because}"),
        )
        .with_recovery(
            "write a directory as an absolute path, or as `~/...` under this person's \
             home directory",
        )
    };
    if text.is_empty() {
        return Err(named("names nothing"));
    }
    if let Some(rest) = text.strip_prefix('~') {
        let rest = rest
            .strip_prefix(['/', '\\'])
            .ok_or_else(|| named("starts with `~` and no separator after it"))?;
        if rest.is_empty() {
            return Err(named("names only the home directory, which is not a shelf"));
        }
        return Ok(home.join(rest));
    }
    let path = PathBuf::from(text);
    if path.is_absolute() {
        return Ok(path);
    }
    Err(named("is not an absolute path"))
}
