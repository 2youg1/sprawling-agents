// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the usage table reads besides the ledger: every shelf's skills
//! and every building's tool servers, read at the moment of asking.

use std::collections::BTreeSet;
use std::path::Path;

use kernel::B3Hash;

use crate::views::lines::buildings_of;
use crate::views::skills::shelf_of;

/// One shelf's copy of a skill, as the scan read it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shelved {
    pub(crate) name: String,
    pub(crate) shelf: wire::SkillShelf,
    pub(crate) digest: B3Hash,
}

/// Every shelf the city keeps or mounts: its stock and the external
/// shelves once, then each building's own shelf.
///
/// `None` when a shelf will not scan or the buildings will not list, for
/// the reason `skills_answer` gives: a broken installation is not an
/// empty one.
pub(crate) fn shelved(city_root: &Path) -> Option<Vec<Shelved>> {
    let home = crate::home::Home::detect().ok()?;
    let copy = |holding: &city::Holding| Shelved {
        name: holding.name.clone(),
        shelf: shelf_of(&holding.shelf),
        digest: holding.hash,
    };
    let mut found: Vec<Shelved> = city::Library::scan(city_root, None, home.path())
        .ok()?
        .all()
        .into_iter()
        .map(copy)
        .collect();
    for building in buildings_of(city_root).ok()? {
        let library = city::Library::scan(city_root, Some(&building), home.path()).ok()?;
        found.extend(
            library
                .all()
                .into_iter()
                .filter(|holding| match holding.shelf {
                    city::Shelf::Building(_) => true,
                    city::Shelf::Library(_) | city::Shelf::External { .. } => false,
                })
                .map(copy),
        );
    }
    Some(found)
}

/// The label of every tool server some building's configuration names.
///
/// A building whose configuration will not load names no server here,
/// as on the MCP page (`mcp_health_answer`): the usage it recorded is in
/// the ledger and still answers, and the configuration page is where its
/// error is shown.
pub(crate) fn configured(city_root: &Path) -> BTreeSet<String> {
    let Ok(buildings) = buildings_of(city_root) else {
        return BTreeSet::new();
    };
    buildings
        .iter()
        .flat_map(|building| match city::load_config(city_root, building) {
            Ok(config) => config
                .mcp
                .iter()
                .map(|server| server.label.as_str().to_owned())
                .collect(),
            Err(_unreadable) => Vec::new(),
        })
        .collect()
}
