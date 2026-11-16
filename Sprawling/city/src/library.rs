// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

mod audit;
mod install;
mod projection;
mod reading;
mod shelf;
mod shipped;

pub use audit::{
    AuditFetchError, AuditReport, AuditRequest, AuditState, HttpAudit, LocalScanner,
    SKILLS_SH_TIMEOUT, SKILLSPECTOR_TIMEOUT, ScannerAudit, audit_skill, audit_state,
};
pub use install::{
    Admitted, Installed, MigrationHooks, Placed, PlannedInstall, Slot, admit_external, install,
    plan_install, skill_digest,
};
pub use projection::materialize;
pub use reading::skill_document;
pub use shelf::disclosure as skill_disclosure;
pub use shelf::{Holding, Shelf};
pub use shipped::{SHIPPED_SECTION, shelve_shipped};

use shelf::OwnShelf;
use shelf::ShelfKey;
use std::path::Path;

use kernel::{Address, AxCode, AxError, B3Hash};

use crate::config_layers::SHELVES_KEY;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    holdings: BTreeMap<ShelfKey, Holding>,
}

impl Library {
    pub fn scan(
        city_root: &Path,
        building: Option<&Address>,
        home: &Path,
        read: &dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
    ) -> Result<Library, AxError> {
        let mut holdings = BTreeMap::new();

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
        reading::shelve(city_root, &OwnShelf::Library, &mut holdings, read)?;
        if let Some(building) = building {
            reading::shelve(
                city_root,
                &OwnShelf::Building(building.clone()),
                &mut holdings,
                read,
            )?;
        }
        Ok(Library { holdings })
    }

    pub fn migrate(
        city_root: &Path,
        building: Option<&Address>,
        mut hooks: MigrationHooks<'_>,
    ) -> Result<Vec<Installed>, AxError> {
        let mut migrated =
            install::migrate_slot(city_root, &Slot::library("migration")?, &mut hooks)?;
        if let Some(building) = building {
            migrated.extend(install::migrate_slot(
                city_root,
                &Slot::building(building, "migration")?,
                &mut hooks,
            )?);
        }
        Ok(migrated)
    }

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

    #[must_use]
    pub fn reading_room(&self, admitted: &[String]) -> Vec<&Holding> {
        let admitted: BTreeSet<ShelfKey> = admitted.iter().map(|name| ShelfKey::of(name)).collect();
        self.all()
            .into_iter()
            .filter(|holding| admitted.contains(&ShelfKey::of(&holding.name)))
            .collect()
    }

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
pub mod test_store {
    use super::*;
    use std::path::PathBuf;

    #[derive(Default)]
    pub struct Store {
        objects: BTreeMap<B3Hash, Vec<u8>>,
    }

    impl Store {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn put(&mut self, bytes: &[u8]) -> B3Hash {
            let digest = B3Hash::digest(bytes);
            self.objects.insert(digest, bytes.to_vec());
            digest
        }

        pub fn read(&self, digest: &B3Hash) -> Result<Vec<u8>, AxError> {
            self.objects.get(digest).cloned().ok_or_else(|| {
                AxError::failure(
                    AxCode::PathNotFound,
                    "read a test CAS object",
                    digest.to_string(),
                )
                .with_recovery("store the object before reading it")
            })
        }

        pub fn migrate(&mut self, city_root: &Path, building: Option<&Address>) -> Vec<Installed> {
            for (root, _) in [
                (
                    kernel::layout::CityLayout::new(city_root).library(),
                    building.is_none(),
                ),
                (
                    building.map_or_else(PathBuf::new, |addr| {
                        kernel::layout::CityLayout::new(city_root).building_skills(addr)
                    }),
                    building.is_some(),
                ),
            ] {
                if !root.exists() {
                    continue;
                }
                for section in reading::read_dir(&root).expect("legacy sections list") {
                    if !section.is_dir() {
                        continue;
                    }
                    for item in reading::read_dir(&section).expect("legacy items list") {
                        if item
                            .file_name()
                            .is_some_and(|name| name == kernel::layout::SHELF_INDEX)
                        {
                            continue;
                        }
                        let inspected =
                            install::precheck::inspect(&item).expect("legacy source reads");
                        self.put(&inspected.stored);
                    }
                }
            }
            let old = self.objects.clone();
            let mut register = |bytes: &[u8]| Ok(self.put(bytes));
            let verify = move |digest: &B3Hash| {
                old.get(digest).cloned().ok_or_else(|| {
                    AxError::failure(
                        AxCode::PathNotFound,
                        "read a test CAS object",
                        digest.to_string(),
                    )
                    .with_recovery("store the old object before migrating")
                })
            };
            Library::migrate(
                city_root,
                building,
                MigrationHooks {
                    register: &mut register,
                    verify: &verify,
                },
            )
            .expect("the legacy shelf migrates")
        }

        pub fn try_scan(
            &self,
            city_root: &Path,
            building: Option<&Address>,
            home: &Path,
        ) -> Result<Library, AxError> {
            Library::scan(city_root, building, home, &|digest| self.read(digest))
        }

        pub fn scan(&self, city_root: &Path, building: Option<&Address>, home: &Path) -> Library {
            self.try_scan(city_root, building, home)
                .expect("the indexed shelves scan")
        }
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
