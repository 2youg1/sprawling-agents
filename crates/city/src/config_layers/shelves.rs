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

use super::Layer;
use super::ladder;

/// The key this module mounts, spelled once for every refusal that
/// names it.
///
/// The serde structure `SkillsSection` in [`super`] decides the
/// spelling; the test at the bottom of this file parses the key as
/// written here, so renaming the field and leaving this constant behind
/// turns that test red.
pub(crate) const SHELVES_KEY: &str = "[skills] shelves";

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
    let layer = ladder::stated(&CityLayout::new(city_root).city_config(), Layer::City)?;
    let Some(stated) = layer.shelves() else {
        return Ok(Vec::new());
    };
    let mut shelves = Vec::with_capacity(stated.len());
    for raw in stated {
        shelves.push(shelf_path(raw, home)?);
    }
    Ok(shelves)
}

/// How one shelf in the city's external shelf list becomes a directory.
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
            format!("`{raw}` in `{SHELVES_KEY}` {because}"),
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::config_layers::ConfigLayer;

    /// The name a refusal shows is the key the parser reads: the probe
    /// is the constant with the table broken onto its own line, so
    /// renaming the serde field and leaving the constant behind stops
    /// it parsing.
    #[test]
    fn the_name_a_refusal_shows_is_the_key_this_build_reads() {
        let probe = format!("{} = []\n", SHELVES_KEY.replace("] ", "]\n"));
        assert!(
            ConfigLayer::parse(&probe).is_ok(),
            "the parser does not read `{SHELVES_KEY}`: {probe}"
        );
    }

    /// The city's own file is the only one that may name a shelf; a
    /// building's or a room's is refused where it was written rather
    /// than read and dropped.
    #[test]
    fn a_shelf_below_the_city_layer_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("CONFIG.toml");
        let document = format!("{} = [\"/elsewhere\"]\n", SHELVES_KEY.replace("] ", "]\n"));
        std::fs::write(&file, document).unwrap();
        let err = ladder::stated(&file, Layer::Building).unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(
            err.recovery().contains("city"),
            "the recovery says where the line belongs: {}",
            err.recovery()
        );
        assert!(
            ladder::stated(&file, Layer::City).is_ok(),
            "the city layer owns this key"
        );
    }
}
