// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the usage table reads besides the ledger: every shelf's skills
//! and every building's tool servers, read at the moment of asking.

use std::collections::BTreeSet;
use std::path::Path;

use kernel::{AxError, B3Hash};

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
/// # Errors
/// Propagates unreadable shelves, buildings and package content. A
/// package's digest is the whole install precheck hash (city D19).
pub(crate) fn shelved(city_root: &Path) -> Result<Vec<Shelved>, AxError> {
    let home = crate::home::Home::detect()?;
    let copy = |holding: &city::Holding| {
        let digest = match &holding.package {
            Some(package) => city::skill_digest(&city_root.join(package.as_str()))?,
            None => holding.hash,
        };
        Ok(Shelved {
            name: holding.name.clone(),
            shelf: shelf_of(&holding.shelf),
            digest,
        })
    };
    let mut found = city::Library::scan(city_root, None, home.path())?
        .all()
        .into_iter()
        .map(copy)
        .collect::<Result<Vec<_>, AxError>>()?;
    for building in buildings_of(city_root)? {
        let library = city::Library::scan(city_root, Some(&building), home.path())?;
        found.extend(
            library
                .all()
                .into_iter()
                .filter(|holding| match holding.shelf {
                    city::Shelf::Building(_) => true,
                    city::Shelf::Library(_) | city::Shelf::External { .. } => false,
                })
                .map(copy)
                .collect::<Result<Vec<_>, AxError>>()?,
        );
    }
    Ok(found)
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
