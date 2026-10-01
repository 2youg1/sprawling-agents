// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The reading room: the skills one building's own file admits, and the
//! names on that list which went nowhere.

use kernel::{Address, AxError};

use super::held;
use crate::worker::workbench::Laying;

impl Laying {
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
        &self,
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

/// One admitted holding into the catalog, by the door its shelf takes.
///
/// A holding inside the city is opened at its address. One on a shelf
/// outside the city has none, and the catalog carries the text the scan
/// read instead (runtime-SPEC.md 8-29-6). In both the hash is what the
/// shelf held at this scan, and the run records it, so a document that
/// changes behind the same name is a difference somebody can see later.
///
/// # Errors
/// Propagates an entry the catalog refuses, and refuses a holding that
/// has neither an address nor its text, which no scan produces.
fn admit_holding(catalog: &mut runtime::Catalog, holding: &city::Holding) -> Result<(), AxError> {
    let entry = |expansion: String| runtime::CatalogEntry {
        name: holding.name.clone(),
        disclosure: holding.disclosure.clone(),
        expansion,
        hash: Some(holding.hash),
        package: holding.package.as_ref().map(|dir| dir.as_str().to_owned()),
    };
    match (holding.shelf.address(), &holding.carried) {
        (Some(at), _) => catalog.admit_skill(entry(at.as_str().to_owned())),
        (None, Some(text)) => catalog.admit_carried_skill(entry(text.clone())),
        (None, None) => Err(AxError::failure(
            kernel::AxCode::InvalidArgs,
            "admit the reading room",
            format!(
                "`{}` has neither an address nor the text it was read with",
                holding.name
            ),
        )
        .with_recovery(
            "report this against city::library: a holding with no address carries its text",
        )),
    }
}
