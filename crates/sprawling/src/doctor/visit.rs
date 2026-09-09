// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Walking a city's buildings and reading each one's bits
//! (sprawling-SPEC.md section 8-48).
//!
//! A building whose rules will not read is an answer about that
//! building, carried back beside the ones that did read. The doctor is
//! never silent: a line saying "this building's rules will not read"
//! is worth more than a report that quietly has one building fewer.

use std::path::Path;

use kernel::{Address, AxCode, AxError};

use super::needs::Bits;

/// What one building answered when visited.
#[derive(Debug)]
pub(crate) enum Visited {
    Bits { building: Address, bits: Bits },
    Unreadable { building: Address, err: AxError },
}

/// Every building of the city, in address order.
///
/// # Errors
/// Refuses a directory that is not a city. One building that will not
/// read is not an error; it is a `Visited::Unreadable` in the list.
pub(crate) fn visit(city_root: &Path) -> Result<Vec<Visited>, AxError> {
    if !crate::assembly::has_history(city_root) {
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

/// The two rule bits and the one configuration bit of a building.
fn bits_of(city_root: &Path, building: &Address) -> Result<Bits, AxError> {
    let rules = city::load(city_root, building)?;
    let config = city::load_config(city_root, building)?;
    Ok(Bits {
        browser: rules.browser(),
        desktop: rules.desktop(),
        shell: config.sandbox.shell,
    })
}

/// The one line a person reads about a building that will not read.
pub(crate) fn unreadable_line(building: &Address, err: &AxError) -> String {
    format!("  {}: its rules will not read: {err}", building.as_str())
}
