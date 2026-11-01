// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Installing one skill into the city's CAS and a shelf's index
//! (`crates/city/spec/Library/Install.lean` §8-28). The source precheck,
//! recheck and refusal paths are unchanged; the landed form is now one
//! CAS object per file, a canonical manifest, and an index row rather
//! than a second directory of bytes.

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, B3Hash, SkillManifest};

use super::reading;
use super::shelf::{Holding, HoldingSource, OwnShelf, ShelfRow, holding_file_name, plain_name};

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

use precheck::{Item, Shape, inspect};

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
                return Err(moved_underneath(
                    &target,
                    "the old shelf changed before its index landed",
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

/// CAS hooks a legacy migration needs: the ordinary write path, and a
/// full verified read of the old whole-package object.
pub struct MigrationHooks<'a> {
    pub register: &'a mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
    pub verify: &'a dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
}

/// Migrates one legacy shelf. The old object must still contain exactly
/// the source bytes before an index can replace its directory.
pub(super) fn migrate_slot(
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

struct Snapshot<'a> {
    shape: &'a Shape,
    stored: &'a [u8],
    manifest: &'a SkillManifest,
    hash: B3Hash,
}

fn store_snapshot(
    snapshot: Snapshot<'_>,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
) -> Result<Admitted, AxError> {
    match snapshot.shape {
        Shape::Document => {
            let expected = snapshot.manifest.document().ok_or_else(|| {
                store_disagrees(&snapshot.hash, &B3Hash::digest(b"missing document"))
            })?;
            let stored = register(snapshot.stored)?;
            if &stored != expected {
                return Err(store_disagrees(expected, &stored));
            }
        }
        Shape::Package(items) => {
            for item in items {
                let Item::File(path, range) = item else {
                    continue;
                };
                let expected = snapshot.manifest.file(path).ok_or_else(|| {
                    store_disagrees(&snapshot.hash, &B3Hash::digest(path.as_bytes()))
                })?;
                let bytes = snapshot
                    .stored
                    .get(range.clone())
                    .ok_or_else(|| store_disagrees(expected, &B3Hash::digest(path.as_bytes())))?;
                let stored = register(bytes)?;
                if &stored != expected {
                    return Err(store_disagrees(expected, &stored));
                }
            }
        }
    }
    let manifest = snapshot.manifest.encode()?;
    let stored = register(&manifest)?;
    if stored != snapshot.hash {
        return Err(store_disagrees(&snapshot.hash, &stored));
    }
    Ok(Admitted {
        hash: stored,
        manifest: snapshot.manifest.clone(),
    })
}

fn document_text(shape: &Shape, stored: &[u8]) -> Result<String, AxError> {
    let bytes = match shape {
        Shape::Document => Some(stored),
        Shape::Package(items) => items.iter().find_map(|item| match item {
            Item::File(path, range) if path == reading::SKILL_FILE => stored.get(range.clone()),
            Item::File(..) | Item::Directory(_) => None,
        }),
    };
    let bytes = bytes.ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read an installed skill",
            "the manifest has no SKILL.md",
        )
        .with_recovery("reinstall the skill with a UTF-8 SKILL.md")
    })?;
    String::from_utf8(bytes.to_vec()).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read an installed skill",
            "SKILL.md is not UTF-8",
        )
        .with_recovery("reinstall the skill with a UTF-8 SKILL.md")
    })
}

/// What the shelf holds under `name`, distinguishing the current index
/// from the legacy directory that migration may replace.
fn shelf_state(slot: &Slot, city_root: &Path, name: &str) -> Result<Shelved, AxError> {
    let root = slot.shelf.root(&CityLayout::new(city_root));
    if let Some(row) = super::shelf::read_index(&root)?
        .into_iter()
        .find(|row| row.name == name)
    {
        if row.section != slot.section {
            return Err(name_taken(name, &row.section));
        }
        if legacy_shelf_state(slot, &root, name)?.is_some() {
            return Err(name_taken(name, "both an index row and a legacy directory"));
        }
        return Ok(Shelved::Current(row.digest));
    }
    let Some(legacy) = legacy_shelf_state(slot, &root, name)? else {
        return Ok(Shelved::Empty);
    };
    Ok(Shelved::Legacy(legacy))
}

fn legacy_shelf_state(slot: &Slot, root: &Path, name: &str) -> Result<Option<B3Hash>, AxError> {
    if !root.exists() {
        return Ok(None);
    }
    for section in reading::read_dir(root)? {
        if !section.is_dir() {
            continue;
        }
        let held_under = reading::spelled(&section)?;
        if !plain_name(&held_under) {
            continue;
        }
        let held: Vec<_> = [section.join(holding_file_name(name)), section.join(name)]
            .into_iter()
            .filter(|named| named.exists())
            .collect();
        match held.as_slice() {
            [] => {}
            [_, _, ..] => {
                let file = holding_file_name(name);
                let twice = format!("{held_under} twice, as {file} and {name}/");
                return Err(name_taken(name, &twice));
            }
            [_] if held_under != slot.section => return Err(name_taken(name, &held_under)),
            [one] => return Ok(Some(inspect(one)?.source_hash)),
        }
    }
    Ok(None)
}

fn require_plain(what: &str, role: &str) -> Result<(), AxError> {
    if plain_name(what) {
        return Ok(());
    }
    Err(AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!("{role} is `{what}`"),
    )
    .with_recovery(
        "name it with letters, digits and dashes: one plain name, no leading dot, no path separators",
    ))
}

fn name_taken(name: &str, held_under: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!("`{name}` is already shelved under {held_under}"),
    )
    .with_recovery(format!(
        "install it under another name, or take the shelved `{name}` off the shelf first: one name holds one holding"
    ))
}

fn moved_underneath(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::VersionConflict,
        "install a skill",
        format!(
            "{} changed while this skill was being installed: {why}",
            path.display()
        ),
    )
    .with_recovery("start the install again from what is there now")
}

fn store_disagrees(ours: &B3Hash, stored: &B3Hash) -> AxError {
    AxError::failure(
        AxCode::CasCorrupt,
        "register an installed skill",
        format!("the store answered {stored} for content that hashes to {ours}"),
    )
    .with_recovery("check the store this install registers with before installing again")
}

fn legacy_io(path: &Path, err: std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "remove a migrated skill directory",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("make the old shelf writable and run the migration again")
}
