// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The reading room: the skills one building's own file admits, and the
//! names on that list which went nowhere.

use kernel::{Address, AxError};

use super::held;
use crate::assembly::RunWorker;

impl RunWorker {
    /// Admits the skills this building's own file names, and says which
    /// of them are not on the shelves.
    ///
    /// The city's shelves may hold a thousand; what costs resident bytes
    /// is the list this building admits. A name on that list which is not
    /// on the shelves is left out rather than promised, and noted so the
    /// person who wrote the name can see it went nowhere.
    ///
    /// # Errors
    /// Propagates a shelf that cannot be read and an entry the catalog
    /// refuses.
    pub(super) fn admit_reading_room(
        &mut self,
        catalog: &std::sync::Arc<std::sync::Mutex<runtime::Catalog>>,
        rules: &city::BuildingRules,
        building: &city::Building,
        addr: &Address,
    ) -> Result<(), AxError> {
        // The reading room, and only it. The city's shelves may hold a
        // thousand skills; what costs resident bytes is the list this
        // building's own file admits, and a name on that list which is
        // not on the shelves is left out rather than promised.
        let home = crate::home::Home::detect()?;
        let shelves = city::Library::scan(&self.city_root, Some(building.addr()), home.path())?;
        for holding in shelves.reading_room(rules.reading_room()) {
            // A catalog entry is opened by an address, and a shelf
            // outside the city holds files that have none. The holding
            // is passed over rather than promised, and noted, so the
            // person who admitted the name can see the run will not
            // read it.
            let Some(at) = holding.shelf.address() else {
                self.note(
                    runtime::diagnostics::Level::Effect,
                    "city::library",
                    &format!(
                        "{} admits `{}`, which is on a shelf outside this city and has no \
                         address a run can open",
                        addr.as_str(),
                        holding.name
                    ),
                );
                continue;
            };
            held(catalog, "admit the reading room")?.admit_skill(runtime::CatalogEntry {
                name: holding.name.clone(),
                disclosure: holding.disclosure.clone(),
                expansion: at.as_str().to_owned(),
                // What the shelf held at this scan. The run records it,
                // so a document that changes content behind the same
                // name is a difference somebody can see later.
                hash: Some(holding.hash),
            })?;
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
