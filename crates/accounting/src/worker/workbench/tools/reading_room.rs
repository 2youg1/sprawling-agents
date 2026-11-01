// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The reading room: the skills one building's own file admits, and the
//! names on that list which went nowhere.

use kernel::{Address, AxCode, AxError, B3Hash};

use super::held;
use crate::worker::workbench::Laying;

impl Laying {
    /// Admits the skills this building's own file names, and says which
    /// of them are not on the shelves.
    pub(super) fn admit_reading_room(
        &self,
        catalog: &std::sync::Arc<std::sync::Mutex<runtime::Catalog>>,
        rules: &city::BuildingRules,
        building: &city::Building,
        addr: &Address,
    ) -> Result<(), AxError> {
        let home = crate::home::Home::detect()?;
        let store = kernel::layout::CityLayout::new(&self.city_root).cas();
        let mut cas = storage::Cas::open(&store).map_err(storage::StorageError::into_ax)?;
        let shelves = {
            let read = |digest: &B3Hash| cas.get(digest).map_err(storage::StorageError::into_ax);
            city::Library::scan(&self.city_root, Some(building.addr()), home.path(), &read)?
        };
        let external = city::city_shelves(&self.city_root, home.path())?;
        for holding in shelves.reading_room(rules.reading_room()) {
            if let city::Shelf::External { index, .. } = holding.shelf {
                let at = usize::try_from(index).map_err(|_| {
                    AxError::failure(
                        AxCode::ConfigInvalid,
                        "admit an external skill",
                        format!("shelf index {index} does not fit this machine"),
                    )
                    .with_recovery("name fewer external shelves in the city's CONFIG.toml")
                })?;
                let root = external.get(at).ok_or_else(|| {
                    AxError::failure(
                        AxCode::ConfigInvalid,
                        "admit an external skill",
                        format!("shelf {index} is no longer configured"),
                    )
                    .with_recovery("restore the shelf entry, then start the run again")
                })?;
                let admitted = city::admit_external(&root.join(&holding.name), &mut |bytes| {
                    cas.put(bytes).map_err(storage::StorageError::into_ax)
                })?;
                if admitted.hash != holding.hash {
                    return Err(AxError::failure(
                        AxCode::VersionConflict,
                        "admit an external skill",
                        format!("`{}` changed between its scan and admission", holding.name),
                    )
                    .with_recovery("start the run again to scan and admit one version"));
                }
            }
            admit_holding(&mut *held(catalog, "admit the reading room")?, holding)?;
        }
        for absent in shelves.missing(rules.reading_room()) {
            self.note(
                runtime::diagnostics::Level::Effect,
                "city::library",
                &format!(
                    "{} admits `{absent}`, which is not on the shelves",
                    addr.as_str()
                ),
            );
        }
        Ok(())
    }
}

/// One admitted holding into the catalog, pinned by manifest digest.
fn admit_holding(catalog: &mut runtime::Catalog, holding: &city::Holding) -> Result<(), AxError> {
    catalog.admit_skill(runtime::CatalogEntry {
        name: holding.name.clone(),
        disclosure: holding.disclosure.clone(),
        expansion: holding.hash.to_string(),
        hash: Some(holding.hash),
    })
}
