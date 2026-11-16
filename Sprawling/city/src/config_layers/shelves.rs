// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError};

use super::Layer;
use super::ladder;

pub(crate) const SHELVES_KEY: &str = "[skills] shelves";

pub(crate) const SKILL_PINS_KEY: &str = "[skills.pins]";

pub fn city_shelves(city_root: &Path, home: &Path) -> Result<Vec<PathBuf>, AxError> {
    let stated = stated_skill_shelves(city_root)?;
    let mut shelves = Vec::with_capacity(stated.len());
    for raw in &stated {
        shelves.push(shelf_path(raw, home)?);
    }
    Ok(shelves)
}

pub fn stated_skill_shelves(city_root: &Path) -> Result<Vec<String>, AxError> {
    let layer = ladder::stated(&CityLayout::new(city_root).city_config(), Layer::City)?;
    Ok(layer.shelves().unwrap_or_default().to_vec())
}

pub fn check_skill_shelves(shelves: &[String], home: &Path) -> Result<(), AxError> {
    for raw in shelves {
        shelf_path(raw, home)?;
    }
    Ok(())
}

fn shelf_path(raw: &str, home: &Path) -> Result<PathBuf, AxError> {
    let text = raw.trim();
    let named = |because: &str| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "mount an external skill shelf",
            format!("`{raw}` in `{SHELVES_KEY}` {because}"),
        )
        .with_recovery(
            "write a directory as an absolute path, or as `~/...` under the User's \
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

    #[test]
    fn the_name_a_refusal_shows_is_the_key_this_build_reads() {
        let probe = format!("{} = []\n", SHELVES_KEY.replace("] ", "]\n"));
        assert!(
            ConfigLayer::parse(&probe).is_ok(),
            "the parser does not read `{SHELVES_KEY}`: {probe}"
        );
    }

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
