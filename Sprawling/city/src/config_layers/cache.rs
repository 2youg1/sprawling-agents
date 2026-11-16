// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[cache]` table: whether a layer keeps a warm prompt cache alive.
//!
//! The setting's spelling and what it plans are `kernel::keep_warm`'s to
//! answer. What this module owns is how one layer's `CONFIG.toml` states
//! it, and what an address settles on when no layer does.
//!
//! Specified by `crates/city/spec/ConfigLayers.lean` §8-4.

use std::path::Path;

use kernel::{Address, AxError, KeepWarm};
use serde::Deserialize;

use super::ConfigLayer;
use super::ladder::Ladder;

/// What one layer's `[cache]` table states. A misspelt setting is
/// refused by the closed set it deserializes into, where the file is
/// parsed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CacheSection {
    pub(super) keep_warm: KeepWarm,
}

/// Whether the prompt cache of a run at `addr` is kept warm: the lowest
/// layer that states it wins, and a ladder that states nothing answers
/// [`KeepWarm::Off`], because a renewal is a request the person pays for
/// and only the person may ask for it.
///
/// # Errors
/// Refuses an address with no building, an unreadable file, and a file
/// that does not parse, exactly as [`super::load`] does.
pub fn keep_warm(city_root: &Path, addr: &Address) -> Result<KeepWarm, AxError> {
    Ok(Ladder::read(city_root, addr)?
        .resolve(ConfigLayer::keep_warm)
        .resolve()
        .copied()
        .unwrap_or(KeepWarm::Off))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::config_layers::{Layer, path};

    fn state(city_root: &Path, room: &Address, layer: Layer, setting: &str) {
        let file = path(city_root, room, layer).unwrap();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, format!("[cache]\nkeep_warm = \"{setting}\"\n")).unwrap();
    }

    #[test]
    fn a_city_that_states_nothing_keeps_no_cache_warm() {
        let dir = tempfile::tempdir().unwrap();
        let room = Address::parse("lab/room1").unwrap();
        assert_eq!(keep_warm(dir.path(), &room).unwrap(), KeepWarm::Off);
    }

    #[test]
    fn the_lowest_layer_that_states_keep_warm_is_what_the_room_keeps() {
        let dir = tempfile::tempdir().unwrap();
        let room = Address::parse("lab/room1").unwrap();
        state(dir.path(), &room, Layer::Building, "five_minute");
        assert_eq!(keep_warm(dir.path(), &room).unwrap(), KeepWarm::FiveMinute);
        state(dir.path(), &room, Layer::Resident, "off");
        assert_eq!(keep_warm(dir.path(), &room).unwrap(), KeepWarm::Off);
    }
}
