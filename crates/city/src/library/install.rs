// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Installing one skill onto a shelf the city keeps itself: the static
//! precheck, the atomic landing, and the content hash registered before
//! it lands (`crates/city/spec/Library/Install.lean` §8-28).
//!
//! Four things happen behind [`install`], and nothing else does:
//! - **A static precheck.** The package is read and judged by shape;
//!   nothing in it is executed, compiled or interpreted. A package
//!   reached through a link is refused whatever the link points at: its
//!   bytes are not the package's to file, and where it points can change
//!   between a check and a landing.
//! - **A name check.** One name holds one holding, so a name taken on
//!   the shelf refuses a different skill under it - in this section or
//!   any other, because the scan keys holdings by name alone.
//! - **An atomic landing.** A document is swapped in whole through
//!   `city::document::replace`, and a package is placed whole through
//!   `city::document::place_tree`: the write paths live in one module.
//! - **A recheck before the landing.** The decision is separated from
//!   the landing ([`plan_install`] then [`PlannedInstall::apply`]) so
//!   that "the package moved underneath while it was being installed" is
//!   a refusal rather than a race; see section 8-28 for the rejected
//!   single-door alternative.
//!
//! The bytes are registered before they land, so a store remembers what
//! was about to arrive even if the arrival fails. The registrar is a
//! parameter because the CAS belongs to `storage` and this crate depends
//! on `kernel` alone - the assembly layer binds `storage::Cas::put`,
//! exactly as `Neighbourhood::scan` receives its inbox counts.

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, B3Hash};

use super::reading;
use super::shelf::{Holding, OwnShelf, holding_file_name, plain_name};
use crate::document::TreeEntry;

mod precheck;
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

/// Where one skill is filed: which of the shelves the city keeps itself,
/// and which section of it.
///
/// These two always travel together - a skill is filed on one shelf,
/// under one section - so they are one value rather than two parameters
/// that can disagree. There is no third constructor: a shelf outside the
/// city is another program's directory mounted read-only (`crates/city/spec/Library.lean`
/// §8-8), so filing a skill on it cannot be asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    shelf: OwnShelf,
    section: String,
}

impl Slot {
    /// A slot on the city's own stock.
    ///
    /// # Errors
    /// Refuses a section that is not one plain directory name: empty,
    /// leading with a dot, or carrying a path separator. The scan skips
    /// dot-prefixed items, so a skill filed under one is a skill no
    /// catalog will ever show.
    pub fn library(section: &str) -> Result<Slot, AxError> {
        Slot::of(OwnShelf::Library, section)
    }

    /// A slot on one building's own shelf.
    ///
    /// # Errors
    /// The same section refusal as [`Slot::library`].
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

    /// Where a holding of `name` in `shape` lands in this slot: the
    /// layout the scan reads back, a document as `<name>.md` and a
    /// package as `<name>/`.
    fn target(&self, layout: &CityLayout, name: &str, shape: &Shape) -> PathBuf {
        let section = self.shelf.root(layout).join(&self.section);
        match shape {
            Shape::Document => section.join(holding_file_name(name)),
            Shape::Package(_) => section.join(name),
        }
    }
}

/// Whether this install had to write anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    /// The bytes were not on the shelf and were landed.
    Fresh,
    /// The same bytes were already shelved under this name: a reinstall
    /// is a no-op rather than a second write.
    AlreadyShelved,
}

/// What an install put on the shelf, and whether it had to.
///
/// The holding is the value a scan of the same shelf reads back, whole:
/// it is read back off the shelf by the scan's own function, so the two
/// cannot disagree. `hash` is what the store files the install under:
/// the document's hash, or the whole package's (`crates/city/spec/Library/Install.lean` §8-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub holding: Holding,
    pub hash: B3Hash,
    pub placed: Placed,
}

/// One install decided but not yet landed.
///
/// Every refusal that can be answered without changing anything on disk
/// is answered in [`plan_install`]; the action can only be taken from a
/// decision, so a landing no one decided cannot be spelled.
#[derive(Debug, Clone)]
pub struct PlannedInstall {
    city_root: PathBuf,
    source: PathBuf,
    slot: Slot,
    name: String,
    shape: Shape,
    /// Every byte the decision was made on, which is what lands.
    stored: Vec<u8>,
    hash: B3Hash,
    /// What the shelf held under the name when the decision was made.
    shelved: Option<B3Hash>,
}

impl PlannedInstall {
    /// The name this skill will be admitted by.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The content hash of the document that will land, which is what a
    /// person approves before it does.
    #[must_use]
    pub fn hash(&self) -> &B3Hash {
        &self.hash
    }

    /// Lands the document, registering its bytes first.
    ///
    /// # Errors
    /// Refuses the whole install when the package or the shelf moved
    /// after the decision (`E_VERSION_CONFLICT`) - nothing on disk is
    /// touched and the registrar is not called. Refuses when the
    /// registrar answers with a hash that is not the content's
    /// (`E_CAS_CORRUPT`), and propagates the registrar's own refusal and
    /// the landing's `E_STORAGE_FATAL`.
    pub fn apply(
        self,
        register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
    ) -> Result<Installed, AxError> {
        // The recheck reads the world again and requires the reading to
        // be the decision's basis. A source that will not even read the
        // same way now is a source that moved: the whole install is
        // refused, whatever shape the difference took, and what the
        // second reading met travels with the refusal.
        let again =
            inspect(&self.source).map_err(|why| moved_underneath(&self.source, why.subject()))?;
        let layout = CityLayout::new(&self.city_root);
        let target = self.slot.target(&layout, &self.name, &self.shape);
        let shelf_again = shelf_state(&self.slot, &self.city_root, &self.name)
            .map_err(|why| moved_underneath(&target, why.subject()))?;
        if again.hash != self.hash {
            return Err(moved_underneath(
                &self.source,
                "what is there now is not what the decision was made against",
            ));
        }
        if shelf_again != self.shelved {
            return Err(moved_underneath(
                &target,
                "what is there now is not what the decision was made against",
            ));
        }
        let placed = match self.shelved {
            Some(_) => Placed::AlreadyShelved,
            None => Placed::Fresh,
        };
        // Registration lands before the swap: the store is the history
        // of what was installed, and a landing that fails after it leaves
        // the store remembering rather than the shelves lying.
        let stored = register(&self.stored)?;
        if stored != self.hash {
            return Err(store_disagrees(&self.hash, &stored));
        }
        if placed == Placed::Fresh {
            self.land(&target)?;
        }
        let holding = reading::holding_at(
            &self.city_root,
            &self.slot.shelf,
            &self.slot.section,
            &target,
        )?
        .ok_or_else(|| not_read_back(&target))?;
        Ok(Installed {
            holding,
            hash: self.hash,
            placed,
        })
    }

    /// Writes the snapshot the decision was made on, never the source
    /// again: bytes swapped in after the recheck cannot reach the shelf.
    fn land(&self, target: &Path) -> Result<(), AxError> {
        let Shape::Package(items) = &self.shape else {
            return crate::document::replace(target, &self.stored);
        };
        let entries = items
            .iter()
            .map(|item| match item {
                Item::Directory(path) => Ok(TreeEntry::Directory(path)),
                Item::File(path, bytes) => self
                    .stored
                    .get(bytes.clone())
                    .map(|body| TreeEntry::File(path, body))
                    .ok_or_else(|| not_read_back(target)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        crate::document::place_tree(target, &entries)
    }
}

/// Decides one install without touching anything: the precheck, the name
/// check, and the state of the world the decision stands on.
///
/// # Errors
/// Refuses a source that is not there (`E_PATH_NOT_FOUND`); a source
/// that is not one skill document (`E_INVALID_ARGS`); a package reached
/// through a link (`E_INVALID_ARGS`, whatever the link points at); a
/// name or section this city cannot file (`E_INVALID_ARGS`); and a name
/// already holding a different skill (`E_INVALID_ARGS`).
pub fn plan_install(
    city_root: &Path,
    slot: &Slot,
    package: &Path,
) -> Result<PlannedInstall, AxError> {
    let inspected = inspect(package)?;
    require_plain(&inspected.name, "the name this skill is admitted by")?;
    let shelved = shelf_state(slot, city_root, &inspected.name)?;
    if shelved.is_some_and(|existing| existing != inspected.hash) {
        return Err(name_taken(&inspected.name, &slot.section));
    }
    Ok(PlannedInstall {
        city_root: city_root.to_path_buf(),
        source: package.to_path_buf(),
        slot: slot.clone(),
        name: inspected.name,
        shape: inspected.shape,
        stored: inspected.stored,
        hash: inspected.hash,
        shelved,
    })
}

/// Reads the whole shelved content using the install precheck's canonical hash.
///
/// # Errors
/// Propagates the precheck's unreadable, oversized or linked content refusal.
pub fn skill_digest(path: &Path) -> Result<B3Hash, AxError> {
    Ok(inspect(path)?.hash)
}

/// The one entry point: decides, then lands.
///
/// # Errors
/// Everything [`plan_install`] and [`PlannedInstall::apply`] refuse.
pub fn install(
    city_root: &Path,
    slot: &Slot,
    package: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>,
) -> Result<Installed, AxError> {
    plan_install(city_root, slot, package)?.apply(register)
}

/// What the shelf holds under `name`, and the refusal when the name is
/// taken under another section - where the scan's by-name key would let
/// two filings silently shadow each other. `<name>.md` and `<name>/` are
/// one name - both at once is refused, since the scan would keep either -
/// and whatever sits there is read by the same precheck as the source,
/// so the two hashes compare like for like.
fn shelf_state(slot: &Slot, city_root: &Path, name: &str) -> Result<Option<B3Hash>, AxError> {
    let root = slot.shelf.root(&CityLayout::new(city_root));
    if !root.exists() {
        return Ok(None);
    }
    let mut found = None;
    for section in reading::read_dir(&root)? {
        if !section.is_dir() {
            continue;
        }
        let held_under = reading::spelled(&section)?;
        if !plain_name(&held_under) {
            continue;
        }
        let held: Vec<_> = [section.join(holding_file_name(name)), section.join(name)]
            .into_iter()
            .filter(|named| {
                !matches!(std::fs::symlink_metadata(named),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound)
            })
            .collect();
        match held.as_slice() {
            [] => {}
            [_, _, ..] => {
                let file = holding_file_name(name);
                let twice = format!("{held_under} twice, as {file} and {name}/");
                return Err(name_taken(name, &twice));
            }
            [_] if held_under != slot.section => return Err(name_taken(name, &held_under)),
            [one] => found = Some(inspect(one)?.hash),
        }
    }
    Ok(found)
}

/// A name or section this city cannot file a skill under: the rule is
/// `shelf::plain_name`, and this is the refusal that states it.
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
        "name it with letters, digits and dashes: one plain name, no leading dot, \
         no path separators",
    ))
}

fn name_taken(name: &str, held_under: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "install a skill",
        format!("`{name}` is already shelved under {held_under}"),
    )
    .with_recovery(format!(
        "install it under another name, or take the shelved `{name}` off the shelf first: \
         one name holds one holding"
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

fn not_read_back(target: &Path) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "install a skill",
        format!(
            "{} landed but does not read back as a holding",
            target.display()
        ),
    )
    .with_recovery("take what landed at the path above off the shelf, then install again")
}

fn store_disagrees(ours: &B3Hash, stored: &B3Hash) -> AxError {
    AxError::failure(
        AxCode::CasCorrupt,
        "register an installed skill",
        format!("the store answered {stored} for content that hashes to {ours}"),
    )
    .with_recovery("check the store this install registers with before installing again")
}
