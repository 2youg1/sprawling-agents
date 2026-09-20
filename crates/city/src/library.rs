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
//! admits is a list a person wrote in `BUILDING.md`.
//!
//! The library lives under the reserved prefix, which is outside every
//! write domain. That is deliberate: a resident may read the stock and
//! may not restock it, so an agent cannot quietly grant itself a skill
//! by writing one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, B3Hash};

/// What a holding is shelved under: the name a reading room admits it
/// by, and nothing else.
///
/// A key rather than a string, and one made in a single place, because
/// the shelves and the reading-room list have to agree on what counts
/// as the same holding. Filing by section and name while admitting by
/// name alone let one name sit on the shelves twice, so a building's
/// own copy of a skill no longer replaced the city's — which is the
/// rule the whole nearer-shelf-wins design rests on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ShelfKey(String);

impl ShelfKey {
    /// The key for a name, whether it came off a shelf or out of a list
    /// a person wrote. Surrounding blanks are not part of a name.
    fn of(name: &str) -> ShelfKey {
        ShelfKey(name.trim().to_owned())
    }
}

/// One shelved item: what it is called, which section it sits in, and
/// the one line a catalog would show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    pub name: String,
    pub section: String,
    /// The first non-empty line of the document, which is what the
    /// author wrote to describe it. Taken rather than generated: a
    /// summary of a summary is a digest, and digests are suspect.
    pub disclosure: String,
    /// What the whole document hashed to when this scan read it.
    ///
    /// Free: the scan already reads every byte to take the disclosure
    /// line, so hashing costs one pass over bytes that are in hand. It
    /// is here rather than computed by a reader because the answer a
    /// reader wants is *whether this changed*, and that question needs
    /// two readings taken at different times — a shelf that reported
    /// only its current contents could never answer it.
    pub hash: B3Hash,
    pub path: PathBuf,
    /// Where this holding sits, as the city spells it. Computed once at
    /// the scan that found the file, because a holding on a building's
    /// own shelf and one on the city's are at different addresses and a
    /// formula over `section` and `name` can only be right about one.
    pub addr: Address,
}

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
    /// # Errors
    /// Propagates a directory that exists and cannot be read — that is
    /// a broken installation rather than an empty one.
    pub fn scan(city_root: &Path, building: Option<&Address>) -> Result<Library, AxError> {
        let layout = CityLayout::new(city_root);
        let mut holdings = BTreeMap::new();
        // The city's shelf first, then the building's over the top of
        // it: the nearer shelf wins, which is the rule the configuration
        // ladder already applies to the values a run is governed by.
        shelve(city_root, &layout.library(), &mut holdings)?;
        if let Some(building) = building {
            shelve(city_root, &layout.building_skills(building), &mut holdings)?;
        }
        Ok(Library { holdings })
    }

    /// Everything on the shelves, in section then name order.
    #[must_use]
    pub fn all(&self) -> Vec<&Holding> {
        let mut shelved: Vec<&Holding> = self.holdings.values().collect();
        shelved.sort_by(|left, right| {
            left.section
                .cmp(&right.section)
                .then(left.name.cmp(&right.name))
        });
        shelved
    }

    /// The sections, which is the navigation a person browses by.
    #[must_use]
    pub fn sections(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = self
            .all()
            .into_iter()
            .map(|holding| holding.section.as_str())
            .collect();
        seen.dedup();
        seen
    }

    /// What one building's reading room admits.
    ///
    /// Matched by name alone, so a person writing the list in
    /// `BUILDING.md` does not have to know which section something was
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

/// Reads one shelf into the map. A shelf that is not there is an empty
/// shelf: most cities start with nothing settled, and most buildings
/// never keep a skill of their own.
///
/// A holding read from a nearer shelf replaces one of the same name
/// read earlier, which is what makes a building's own copy of a skill
/// the one its residents get.
///
/// # Errors
/// Propagates a directory that exists and cannot be read, a holding
/// that cannot be read, a name this machine spells outside Unicode, and
/// a path the city cannot spell as an address.
fn shelve(
    city_root: &Path,
    root: &Path,
    holdings: &mut BTreeMap<ShelfKey, Holding>,
) -> Result<(), AxError> {
    if !root.exists() {
        return Ok(());
    }
    for section in read_dir(root)? {
        if !section.is_dir() {
            continue;
        }
        let section_name = spelled(&section)?;
        for item in read_dir(&section)? {
            if item.is_dir() || item.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let file = spelled(&item)?;
            let Some(name) = file.strip_suffix(".md").map(str::to_owned) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let text = std::fs::read_to_string(&item).map_err(|err| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "read a library holding",
                    format!("{}: {err}", item.display()),
                )
                .with_recovery(
                    "give this process permission to read the file named above, or \
                     take it off the shelf, then scan again",
                )
            })?;
            // The address is read back off the path the layout chose,
            // so a shelf that moves cannot leave the addresses of what
            // sits on it pointing at where it used to be.
            let addr = address_of(city_root, &item)?;
            holdings.insert(
                ShelfKey::of(&name),
                Holding {
                    name,
                    section: section_name.clone(),
                    disclosure: first_line(&text),
                    hash: B3Hash::digest(text.as_bytes()),
                    path: item,
                    addr,
                },
            );
        }
    }
    Ok(())
}

/// How the city addresses a file on one of its shelves.
fn address_of(city_root: &Path, item: &Path) -> Result<Address, AxError> {
    let relative = item.strip_prefix(city_root).map_err(|_| {
        AxError::failure(
            AxCode::StorageFatal,
            "address a library holding",
            format!("{} is outside {}", item.display(), city_root.display()),
        )
        .with_recovery("scan the library of the city the holding sits in")
    })?;
    let mut spelled_out = Vec::new();
    for component in relative.components() {
        spelled_out.push(
            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| unspellable(item))?,
        );
    }
    Address::parse(&spelled_out.join("/"))
}

/// The last element of a path, as the city has to be able to write it.
///
/// A name this machine spells outside Unicode is reported rather than
/// dropped: a skill silently absent from every reading room is the one
/// failure a person cannot see from the catalog.
fn spelled(path: &Path) -> Result<String, AxError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| unspellable(path))
}

fn unspellable(path: &Path) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "read the library",
        format!("{} is not named in Unicode", path.display()),
    )
    .with_recovery("rename it to letters, digits and dashes, then scan again")
}

fn read_dir(path: &Path) -> Result<Vec<PathBuf>, AxError> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(path).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the library",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "give this process permission to list the directory named above, then \
             scan again",
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the library",
                format!("{}: {err}", path.display()),
            )
            .with_recovery(
                "give this process permission to list the directory named above, \
                 then scan again",
            )
        })?;
        out.push(entry.path());
    }
    out.sort();
    Ok(out)
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .trim_start_matches(['#', ' '])
        .to_owned()
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
