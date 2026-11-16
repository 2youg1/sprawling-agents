// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxCode, AxError};

use super::needs::Bits;

#[derive(Debug)]
pub(crate) enum Visited {
    Bits { building: Address, bits: Bits },
    Unreadable { building: Address, err: AxError },
}

pub(crate) fn visit(city_root: &Path) -> Result<Vec<Visited>, AxError> {
    if city::has_history(city_root)? == city::History::Absent {
        return Err(AxError::failure(
            AxCode::PathNotFound,
            "visit a city",
            format!("{}: no city here", city_root.display()),
        )
        .with_recovery("name the directory a city was raised in"));
    }
    let buildings = city::buildings(city_root)?;
    Ok(buildings
        .into_iter()
        .map(|building| match bits_of(city_root, &building) {
            Ok(bits) => Visited::Bits { building, bits },
            Err(err) => Visited::Unreadable { building, err },
        })
        .collect())
}

fn bits_of(city_root: &Path, building: &Address) -> Result<Bits, AxError> {
    let rules = city::load(city_root, building)?;
    let config = city::load_config(city_root, building)?;
    Ok(Bits {
        browser: rules.browser(),
        shell: config.sandbox.shell,
        interpreter: config.sandbox.interpreter,
    })
}

pub(crate) fn unreadable_line(building: &Address, err: &AxError) -> String {
    format!("  {}: its rules will not read: {err}", building.as_str())
}
