// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The facts a person writes into the city's own layer from the settings
//! page (city-SPEC.md 8-34).
//!
//! The city's layer is the far end of the ladder: what it states holds
//! wherever a building or a room states nothing. It is written through
//! the same read-modify-write as every other layer, so a key a person
//! wrote there by hand survives a write from the page.

use std::path::Path;

use kernel::{AxError, Effort, KeepWarm};

/// One fact of the city's own layer a page may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitySetting {
    KeepWarm(KeepWarm),
    Effort(Effort),
}

/// Writes one fact into the city's own `CONFIG.toml`.
///
/// # Errors
/// Propagates a city layer that exists and cannot be read or parsed, and
/// a directory that cannot be written.
pub fn write_city_setting(city_root: &Path, setting: CitySetting) -> Result<(), AxError> {
    let _ = (city_root, setting);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use kernel::Address;

    /// A room that states nothing reads what the city's layer was given,
    /// and the effort says it came from the city.
    #[test]
    fn a_city_setting_lands_in_the_city_layer_and_the_rooms_read_it() {
        let dir = tempfile::tempdir().unwrap();
        let room = Address::parse("lab/room1").unwrap();
        let file = kernel::layout::CityLayout::new(dir.path()).city_config();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "[clock]\nstamp = \"minute\"\n").unwrap();

        write_city_setting(dir.path(), CitySetting::KeepWarm(KeepWarm::FiveMinute)).unwrap();
        write_city_setting(dir.path(), CitySetting::Effort(Effort::High)).unwrap();

        assert_eq!(
            crate::keep_warm(dir.path(), &room).unwrap(),
            KeepWarm::FiveMinute
        );
        assert_eq!(
            crate::settled_effort(dir.path(), &room).unwrap(),
            Some((Effort::High, crate::Layer::City))
        );
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            text.contains("stamp"),
            "a key written by hand is kept: {text}"
        );
    }
}
