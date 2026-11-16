// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use kernel::{AxError, B3Hash};

use crate::views::lines::buildings_of;
use crate::views::skills::shelf_of;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shelved {
    pub(crate) name: String,
    pub(crate) shelf: wire::SkillShelf,
    pub(crate) digest: B3Hash,
}

pub(crate) fn shelved(city_root: &Path) -> Result<Vec<Shelved>, AxError> {
    let home = crate::home::Home::detect()?;
    let store = kernel::layout::CityLayout::new(city_root).cas();
    let cas = storage::Cas::open(&store).map_err(storage::StorageError::into_ax)?;
    let read = |digest: &B3Hash| cas.get(digest).map_err(storage::StorageError::into_ax);
    let copy = |holding: &city::Holding| Shelved {
        name: holding.name.clone(),
        shelf: shelf_of(&holding.shelf),
        digest: holding.hash,
    };
    let mut found = city::Library::scan(city_root, None, home.path(), &read)?
        .all()
        .into_iter()
        .map(copy)
        .collect::<Vec<_>>();
    for building in buildings_of(city_root)? {
        let library = city::Library::scan(city_root, Some(&building), home.path(), &read)?;
        found.extend(
            library
                .all()
                .into_iter()
                .filter(|holding| match holding.shelf {
                    city::Shelf::Building(_) => true,
                    city::Shelf::Library | city::Shelf::External { .. } => false,
                })
                .map(copy)
                .collect::<Vec<_>>(),
        );
    }
    Ok(found)
}

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
