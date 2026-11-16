// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, B3Hash};

use super::super::reading;
use super::super::shelf::plain_name;
use super::{Installed, Shelved, Slot, plan_install};

/// CAS hooks a legacy migration needs: the ordinary write path, and a
/// full verified read of the old whole-package object.
pub struct MigrationHooks<'a> {
    pub register: &'a mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
    pub verify: &'a dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
}

/// Migrates one legacy shelf. The old object must still contain exactly
/// the source bytes before an index can replace its directory.
pub fn migrate_slot(
    city_root: &Path,
    slot: &Slot,
    hooks: &mut MigrationHooks<'_>,
) -> Result<Vec<Installed>, AxError> {
    let root = slot.shelf.root(&CityLayout::new(city_root));
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut migrated = Vec::new();
    for section in reading::read_dir(&root)? {
        if !section.is_dir() {
            continue;
        }
        let section_name = reading::spelled(&section)?;
        if !plain_name(&section_name) {
            continue;
        }
        let slot = Slot::of(slot.shelf.clone(), &section_name)?;
        for item in reading::read_dir(&section)? {
            let item_name = reading::spelled(&item)?;
            if !plain_name(&item_name) || item_name == kernel::layout::SHELF_INDEX {
                continue;
            }
            let planned = plan_install(city_root, &slot, &item)?;
            let previous = match &planned.shelved {
                Shelved::Legacy(previous) => *previous,
                Shelved::Empty | Shelved::Current(_) => continue,
            };
            let old = (hooks.verify)(&previous)?;
            if old != planned.stored {
                return Err(AxError::failure(
                    AxCode::CasCorrupt,
                    "migrate a skill shelf",
                    format!(
                        "{} does not match the CAS object installed with it",
                        item.display()
                    ),
                )
                .with_recovery(
                    "restore the old CAS object or the shelf bytes, then open the city again",
                ));
            }
            migrated.push(planned.apply(&mut *hooks.register)?);
        }
    }
    Ok(migrated)
}
