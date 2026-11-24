// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::config::ExposureTable;
use kernel::layout::CityLayout;
use kernel::{AxError, Effort, KeepWarm};

use super::write::{Change, change_at};

pub fn stated_city_config(city_root: &Path) -> Result<super::ConfigLayer, AxError> {
    let path = kernel::layout::CityLayout::new(city_root).city_config();
    super::ladder::stated(&path, super::Layer::City)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitySetting<'a> {
    KeepWarm(KeepWarm),
    Effort(Effort),

    Catalog(&'a ExposureTable),

    SkillShelves(&'a [String]),
}

pub fn write_city_setting(city_root: &Path, setting: CitySetting<'_>) -> Result<(), AxError> {
    let change = match setting {
        CitySetting::KeepWarm(setting) => Change::KeepWarm(setting),
        CitySetting::Effort(effort) => Change::Effort(effort),
        CitySetting::Catalog(table) => {
            super::catalog::check(table)?;
            Change::Catalog(table)
        }
        CitySetting::SkillShelves(shelves) => Change::SkillShelves(shelves),
    };
    change_at(&CityLayout::new(city_root).city_config(), change)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use kernel::Address;

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
