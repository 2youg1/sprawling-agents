// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Installing one skill into the city's CAS and a shelf's index
//! (`Sprawling/city/spec/Library/Install.lean` §8-28). The source precheck,
//! recheck and refusal paths are unchanged; the landed form is now one
//! CAS object per file, a canonical manifest, and an index row rather
//! than a second directory of bytes.

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, B3Hash, SkillManifest};

use super::shelf::{Holding, HoldingSource, OwnShelf, ShelfRow, holding_file_name};

mod migrate;

pub use migrate::MigrationHooks;
pub(super) use migrate::migrate_slot;
pub(super) mod precheck;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::type_complexity,
    reason = "test code"
)]
mod tests;

use precheck::{Shape, inspect};

mod store;

use store::{
    Snapshot, document_text, legacy_io, moved_underneath, require_plain, shelf_state,
    store_snapshot,
};

/// Where one skill is indexed: which shelf and which section. There is
/// no constructor for an external shelf, which remains read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    shelf: OwnShelf,
    section: String,
}

impl Slot {
    pub fn library(section: &str) -> Result<Slot, AxError> {
        Slot::of(OwnShelf::Library, section)
    }

    pub fn building(building: &Address, section: &str) -> Result<Slot, AxError> {
        Slot::of(OwnShelf::Building(building.clone()), section)
    }

    fn of(shelf: OwnShelf, section: &str) -> Result<Slot, AxError> {
        require_plain(section, "the section this skill is filed under")?;
        Ok(Slot {
            shelf,
            section: section.to_owned(),
        })
    }
}

/// Whether this install had to write a new version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    Fresh,
    AlreadyShelved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub holding: Holding,
    pub hash: B3Hash,
    pub placed: Placed,
    /// The pre-manifest digest, present only for a migrated legacy shelf.
    pub previous: Option<B3Hash>,
}

/// The CAS result of an admission that needs no shelf index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub hash: B3Hash,
    pub manifest: SkillManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Shelved {
    Empty,
    Current(B3Hash),
    Legacy(B3Hash),
}

/// One install decided but not yet written.
#[derive(Debug, Clone)]
pub struct PlannedInstall {
    city_root: PathBuf,
    source: PathBuf,
    slot: Slot,
    name: String,
    shape: Shape,
    stored: Vec<u8>,
    source_hash: B3Hash,
    manifest: SkillManifest,
    hash: B3Hash,
    shelved: Shelved,
}

impl PlannedInstall {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn hash(&self) -> &B3Hash {
        &self.hash
    }

    /// Writes the decided snapshot, then its index. A current index with
    /// the same digest is already settled; a different digest is a new
    /// candidate version, not an overwrite of any building's pin.
    pub fn apply(
        self,
        register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
    ) -> Result<Installed, AxError> {
        self.recheck()?;
        if self.shelved == Shelved::Current(self.hash) {
            return Ok(Installed {
                holding: self.holding()?,
                hash: self.hash,
                placed: Placed::AlreadyShelved,
                previous: None,
            });
        }
        let admitted = self.store(register)?;
        let root = self.slot.shelf.root(&CityLayout::new(&self.city_root));
        super::shelf::upsert_index(
            &root,
            ShelfRow {
                section: self.slot.section.clone(),
                name: self.name.clone(),
                digest: admitted.hash,
            },
        )?;
        if let Shelved::Legacy(previous) = self.shelved {
            self.remove_legacy(previous)?;
        }
        let placed = Placed::Fresh;
        Ok(Installed {
            holding: self.holding()?,
            hash: admitted.hash,
            placed,
            previous: match self.shelved {
                Shelved::Legacy(previous) => Some(previous),
                Shelved::Empty | Shelved::Current(_) => None,
            },
        })
    }

    fn recheck(&self) -> Result<(), AxError> {
        let again =
            inspect(&self.source).map_err(|why| moved_underneath(&self.source, why.subject()))?;
        if again.source_hash != self.source_hash {
            return Err(moved_underneath(
                &self.source,
                "what is there now is not what the decision was made against",
            ));
        }
        let state = shelf_state(&self.slot, &self.city_root, &self.name)
            .map_err(|why| moved_underneath(&self.source, why.subject()))?;
        if state != self.shelved {
            return Err(moved_underneath(
                &self.source,
                "the shelf changed while this skill was being installed",
            ));
        }
        Ok(())
    }

    fn store(
        &self,
        register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
    ) -> Result<Admitted, AxError> {
        store_snapshot(
            Snapshot {
                shape: &self.shape,
                stored: &self.stored,
                manifest: &self.manifest,
                hash: self.hash,
            },
            register,
        )
    }

    fn holding(&self) -> Result<Holding, AxError> {
        let text = document_text(&self.shape, &self.stored)?;
        Ok(Holding::of(HoldingSource {
            name: self.name.clone(),
            section: self.slot.section.clone(),
            text: &text,
            shelf: self.slot.shelf.as_shelf(),
            hash: self.hash,
        }))
    }

    fn remove_legacy(&self, previous: B3Hash) -> Result<(), AxError> {
        let root = self.slot.shelf.root(&CityLayout::new(&self.city_root));
        let section = root.join(&self.slot.section);
        let targets = [
            section.join(holding_file_name(&self.name)),
            section.join(&self.name),
        ];
        for target in targets {
            if !target.exists() {
                continue;
            }
            if inspect(&target)?.source_hash != previous {
                // The index row has already landed, so every later
                // install of this name refuses "both an index row and a
                // legacy directory" until the directory is gone: a
                // retry cannot recover, and the recovery says what
                // actually unblocks the name.
                return Err(AxError::failure(
                    AxCode::VersionConflict,
                    "remove a migrated skill directory",
                    format!("{} changed before its index landed", target.display()),
                )
                .with_recovery(
                    "the index row already names the new version; remove the old directory by \
                     hand, then install again",
                ));
            }
            if target.is_dir() {
                std::fs::remove_dir_all(&target).map_err(|err| legacy_io(&target, err))?;
            } else {
                std::fs::remove_file(&target).map_err(|err| legacy_io(&target, err))?;
            }
        }
        Ok(())
    }
}

/// Decides one install without changing anything.
pub fn plan_install(
    city_root: &Path,
    slot: &Slot,
    package: &Path,
) -> Result<PlannedInstall, AxError> {
    let inspected = inspect(package)?;
    require_plain(&inspected.name, "the name this skill is admitted by")?;
    let shelved = shelf_state(slot, city_root, &inspected.name)?;
    Ok(PlannedInstall {
        city_root: city_root.to_path_buf(),
        source: package.to_path_buf(),
        slot: slot.clone(),
        name: inspected.name,
        shape: inspected.shape,
        stored: inspected.stored,
        source_hash: inspected.source_hash,
        manifest: inspected.manifest,
        hash: inspected.hash,
        shelved,
    })
}

/// The one entry point: decides, then writes CAS and the index.
pub fn install(
    city_root: &Path,
    slot: &Slot,
    package: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
) -> Result<Installed, AxError> {
    plan_install(city_root, slot, package)?.apply(register)
}

/// Copies an external skill into CAS on admission, without adding it to
/// any shelf. Its candidate manifest already carries every file digest.
pub fn admit_external(
    package: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
) -> Result<Admitted, AxError> {
    let inspected = inspect(package)?;
    let again = inspect(package)?;
    if again.source_hash != inspected.source_hash {
        return Err(moved_underneath(
            package,
            "the external skill changed while it was being admitted",
        ));
    }
    store_snapshot(
        Snapshot {
            shape: &inspected.shape,
            stored: &inspected.stored,
            manifest: &inspected.manifest,
            hash: inspected.hash,
        },
        register,
    )
}

/// The manifest digest of a source, without landing it.
pub fn skill_digest(path: &Path) -> Result<B3Hash, AxError> {
    Ok(inspect(path)?.hash)
}
