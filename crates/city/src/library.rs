// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city's central stock of settled work, and the reading room each
//! building takes a subset of it into.
//!
//! The division is the only reason resident context does not grow with
//! the disk. A thousand skills may sit in the library and all of them
//! are findable; **not one byte of them enters a run's prefix** unless
//! that building's reading room admits it, and what a reading room
//! admits is a list a person wrote in `RULES.toml`.
//!
//! The library lives under the reserved prefix, which is outside every
//! write domain. That is deliberate: a resident may read the stock and
//! may not restock it, so an agent cannot quietly grant itself a skill
//! by writing one.
//!
//! **One scan reads every shelf there is**: the city's stock, the
//! building's own shelf, and the directories the city's configuration
//! mounts read-only from elsewhere on this machine. A name is filed
//! once, so a skill kept in the library and one kept by another program
//! are one row, and which of them a run gets is the shelf order rather
//! than a reader's preference.

use std::collections::{BTreeMap, BTreeSet};

mod reading;
mod shelf;

pub use shelf::{Holding, Shelf};

use reading::OwnShelf;
use shelf::ShelfKey;
use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError};

use crate::config_layers::SHELVES_KEY;

/// The city's stock, one entry per name.
///
/// One entry per name rather than per section and name: a reading room
/// admits by name, so two holdings under one name would be two answers
/// to one question and the nearer shelf would win only sometimes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    holdings: BTreeMap<ShelfKey, Holding>,
}

impl Library {
    /// Reads the stock from disk.
    ///
    /// A city with no library is an empty library, not an error: most
    /// cities start with nothing settled.
    ///
    /// `home` is this person's home directory, which the city's own
    /// configuration may name a shelf under. It is a parameter because
    /// reading the environment is not this crate's job, and a scan in a
    /// test then reads the directories the test made.
    ///
    /// # Errors
    /// Refuses a shelf that exists and is not a directory, and
    /// propagates a directory that exists and cannot be read - that is
    /// a broken installation rather than an empty one.
    pub fn scan(
        city_root: &Path,
        building: Option<&Address>,
        home: &Path,
    ) -> Result<Library, AxError> {
        let layout = CityLayout::new(city_root);
        let mut holdings = BTreeMap::new();
        // The farthest shelf first, so that the nearer one replaces it
        // under one name: the city's own stock is written by whoever
        // keeps this city and beats a directory belonging to another
        // program, and the building's own shelf beats the city's. That
        // is the rule the configuration ladder already applies to the
        // values a run is governed by.
        for (index, root) in crate::config_layers::city_shelves(city_root, home)?
            .into_iter()
            .enumerate()
        {
            let index = u32::try_from(index).map_err(|_| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "mount an external skill shelf",
                    "the city names more shelves than an index holds",
                )
                .with_recovery(format!("name fewer shelves in `{SHELVES_KEY}`"))
            })?;
            reading::shelve_external(index, &root, &mut holdings)?;
        }
        reading::shelve(
            city_root,
            &layout.library(),
            OwnShelf::Library,
            &mut holdings,
        )?;
        if let Some(building) = building {
            reading::shelve(
                city_root,
                &layout.building_skills(building),
                OwnShelf::Building,
                &mut holdings,
            )?;
        }
        Ok(Library { holdings })
    }

    /// Everything on the shelves, in the order a catalog lists them:
    /// the city's stock first, then the building's own shelf, then the
    /// external shelves in the order the city's configuration names
    /// them; within a shelf, section then name order.
    ///
    /// The shelf decides that order, not the section: an external
    /// holding has no section to sort under, so sorting every holding
    /// by section once put the directories of other programs above the
    /// city's own stock.
    #[must_use]
    pub fn all(&self) -> Vec<&Holding> {
        let mut shelved: Vec<&Holding> = self.holdings.values().collect();
        shelved.sort_by(|left, right| {
            left.shelf
                .catalog_position()
                .cmp(&right.shelf.catalog_position())
                .then_with(|| left.section.cmp(&right.section))
                .then_with(|| left.name.cmp(&right.name))
        });
        shelved
    }

    /// The sections, which is the navigation a person browses by.
    ///
    /// One entry per section, in the order the catalog lists its first
    /// holding: two shelves may file under the same section name, so
    /// removing only the neighbours would leave that name twice.
    #[must_use]
    pub fn sections(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for holding in self.all() {
            let section = holding.section.as_str();
            if !seen.contains(&section) {
                seen.push(section);
            }
        }
        seen
    }

    /// What one building's reading room admits.
    ///
    /// Matched by name alone, so a person writing the list in
    /// `RULES.toml` does not have to know which section something was
    /// filed under. A name on the list that is not on the shelves is
    /// simply absent from the result: the catalog shows what a run can
    /// actually reach, and a promise of a missing skill is worse than
    /// its absence.
    #[must_use]
    pub fn reading_room(&self, admitted: &[String]) -> Vec<&Holding> {
        let admitted: BTreeSet<ShelfKey> = admitted.iter().map(|name| ShelfKey::of(name)).collect();
        self.all()
            .into_iter()
            .filter(|holding| admitted.contains(&ShelfKey::of(&holding.name)))
            .collect()
    }

    /// The names on a list that the shelves do not have. Shown to the
    /// person who wrote the list, since only they can fix it.
    #[must_use]
    pub fn missing(&self, admitted: &[String]) -> Vec<String> {
        admitted
            .iter()
            .filter(|name| !self.holdings.contains_key(&ShelfKey::of(name)))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
