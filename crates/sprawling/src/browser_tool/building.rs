// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The building's own browser - this city starts it, and its profile
//! is the building's - and the one call that says which browser tools a
//! building's rules ask for, in the order the catalogue hashes.

use kernel::{AxCode, AxError};
use memory::Cas;

use super::BrowserTool;
use super::Role;
use super::person::for_user_browser;

/// What the model is told about the building's own browser.
pub(super) const BUILDING_DISCLOSURE: &str = "drive this machine's browser: open a page, look at it, act on it, \
     take a screenshot, measure boxes, read the console, resize, close";

/// Builds the tool a building with `browser: true` gets.
///
/// The profile is the building's own, so what a browser remembers - a
/// login, a cookie, a permission - belongs to one line of business
/// rather than to the machine. The port is derived from the city
/// directory, so two cities on one machine drive two browsers.
///
/// # Errors
/// Propagates a profile path this city cannot make, and a content store
/// that will not open.
pub(crate) fn for_building(
    city_root: &std::path::Path,
    building: &kernel::Address,
) -> Result<BrowserTool, AxError> {
    let profile = city_root.join(browser::Profile::of(building)?.path().as_str());
    std::fs::create_dir_all(&profile).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "make a browser profile",
            format!("{}: {err}", profile.display()),
        )
        .with_recovery("fix the directory's permissions")
    })?;
    let cas = open_cas(city_root)?;
    let port = crate::browser_bidi::port_for(city_root);
    BrowserTool::new(
        // A window a person can watch is the default; `-headless` is on
        // the launch plan for the caller that asks for it.
        Role::Building,
        Box::new(crate::browser_bidi::LazyEngine::new(profile, false, port)),
        cas,
    )
}

pub(super) fn open_cas(city_root: &std::path::Path) -> Result<Cas, AxError> {
    Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
        .map_err(memory::MemoryError::into_ax)
}

/// Every browser tool this building's rules ask for, in table order.
///
/// One call rather than a branch at the registration site: the order is
/// the catalogue's (the resident segment is hashed with it), and a
/// building's own browser comes before the person's. A confidential
/// building asks for neither - `city::policy` refuses both settings.
///
/// # Errors
/// Propagates a profile path this city cannot make and a content store
/// that will not open.
pub(crate) fn for_rules(
    city_root: &std::path::Path,
    building: &kernel::Address,
    rules: &city::BuildingRules,
) -> Result<Vec<BrowserTool>, AxError> {
    let mut tools = Vec::new();
    if rules.browser() {
        tools.push(for_building(city_root, building)?);
    }
    if let Some(user) = rules.usersbrowser() {
        tools.push(for_user_browser(city_root, user)?);
    }
    Ok(tools)
}
