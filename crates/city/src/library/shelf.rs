// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a shelf holds, and where the library files it.
//!
//! The value types the scan produces, kept apart from the scanning:
//! what a holding is does not change when a shelf is read differently,
//! and the address a page opens one by is the whole of the answer this
//! module owns.
//!
//! Specified by `crates/city/spec/Library.lean` §8-8.

use kernel::layout::CityLayout;
use kernel::{Address, B3Hash};

use std::path::PathBuf;

/// The extension a shelved document carries: one skill is one document,
/// filed as `<name>.md`. Both directions of that spelling live here, so
/// the scan that reads a shelf and the install that writes one cannot
/// disagree about what a holding's file is called.
pub(super) const HOLDING_EXT: &str = "md";

/// The name a shelved file is filed under, or `None` when the file is
/// not a holding.
pub(super) fn holding_name(file_name: &str) -> Option<String> {
    let name = file_name.strip_suffix(HOLDING_EXT)?;
    let name = name.strip_suffix('.')?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_owned())
}

/// The file name a holding of `name` is shelved as.
pub(super) fn holding_file_name(name: &str) -> String {
    format!("{name}.{HOLDING_EXT}")
}

/// Whether `name` is a name something can be filed under: non-empty,
/// not starting with a dot, and one path segment.
///
/// The one rule both directions answer to: the scan skips what it
/// rejects and the install refuses it, so a half-written staging file
/// beside a document and a name no catalog would ever show are one rule
/// seen from two sides rather than two rules that can drift.
pub(super) fn plain_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\'])
}

/// Which of the shelves the city keeps itself.
///
/// A shelf outside the city is another program's directory mounted
/// read-only (section 8-8), so there is no arm for one here: filing a
/// skill on it is something that cannot be asked for rather than
/// something refused. The building arm carries the address because the
/// shelf is that building's own subtree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OwnShelf {
    Library,
    Building(Address),
}

impl OwnShelf {
    /// Where this shelf's root sits in the city.
    pub(super) fn root(&self, layout: &CityLayout) -> PathBuf {
        match self {
            OwnShelf::Library => layout.library(),
            OwnShelf::Building(building) => layout.building_skills(building),
        }
    }

    /// Where a document read off this shelf sits.
    pub(super) fn at(&self, item: Address) -> Shelf {
        match self {
            OwnShelf::Library => Shelf::Library(item),
            OwnShelf::Building(_) => Shelf::Building(item),
        }
    }
}

/// What a holding is shelved under: the name a reading room admits it
/// by, and nothing else.
///
/// A key rather than a string, and one made in a single place, because
/// the shelves and the reading-room list have to agree on what counts
/// as the same holding. Filing by section and name while admitting by
/// name alone would let one name sit on the shelves twice, so a
/// building's own copy of a skill would not replace the city's — which
/// is the rule the whole nearer-shelf-wins design rests on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ShelfKey(String);

impl ShelfKey {
    /// The key for a name, whether it came off a shelf or out of a list
    /// a person wrote. Surrounding blanks are not part of a name.
    pub(super) fn of(name: &str) -> ShelfKey {
        ShelfKey(name.trim().to_owned())
    }
}

/// Which shelf a holding came from, and where its document sits.
///
/// One value rather than a shelf name beside a path, because a holding
/// is on one shelf and at one place; two fields would let them disagree,
/// and a holding that said `Library` while pointing outside the city is
/// a state no shelf can be in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shelf {
    /// The city's central stock, under the reserved prefix.
    Library(Address),
    /// This building's own shelf, inside the building.
    Building(Address),
    /// A shelf outside the city that the city's configuration names,
    /// mounted read-only: which shelf of that list this is, and the
    /// document's path from the shelf's root.
    ///
    /// There is no address here, and inventing one would state a reach
    /// the file does not have: every address is a path under the city
    /// root, and this file is not under it.
    External { index: u32, path: String },
}

impl Shelf {
    /// The address a reader can open this holding by, where it has one.
    ///
    /// The question a catalog asks before it promises a skill to a run:
    /// an entry is opened by an address, and a holding outside the city
    /// has none.
    #[must_use]
    pub fn address(&self) -> Option<&Address> {
        match self {
            Shelf::Library(addr) | Shelf::Building(addr) => Some(addr),
            Shelf::External { .. } => None,
        }
    }

    /// Where this shelf sits in the order a catalog lists shelves: the
    /// city's stock first, then the building's own copy, then the
    /// external shelves in the order the city's configuration names
    /// them.
    ///
    /// The arms above are that order, and the match below derives the
    /// position from them: a shelf added later cannot build until it is
    /// placed, so the enum stays the only place the order is decided.
    #[must_use]
    pub(super) fn catalog_position(&self) -> (u8, u32) {
        match self {
            Shelf::Library(_) => (0, 0),
            Shelf::Building(_) => (1, 0),
            Shelf::External { index, .. } => (2, *index),
        }
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
    /// two readings taken at different times - a shelf that reported
    /// only its current contents could never answer it.
    pub hash: B3Hash,
    /// Which shelf, and where on it. One value, so a reader opening the
    /// document and a page naming the shelf read one fact.
    pub shelf: Shelf,
    /// The package directory, when the holding is filed as one: the
    /// scan says so here, so a reader never guesses it from a document
    /// that happens to be called `SKILL.md`. `None` for a single
    /// document, and for a shelf outside the city, which has no address.
    pub package: Option<Address>,
    /// The document's text, for a holding no run can open by an address:
    /// one on a shelf outside the city. The scan read these bytes to take
    /// the disclosure line and the hash, so the text a reading room hands
    /// a run is the text `hash` names (city D9). `None`
    /// for a holding on the city's own shelves, which a run opens where
    /// it sits.
    pub carried: Option<String>,
}

impl Holding {
    /// One holding, derived from the document's bytes in exactly one
    /// place. The disclosure line, the content hash and the text carried
    /// for a shelf with no address are what every scan and every install
    /// reports, and two derivations of them would be two answers to what
    /// this document said.
    pub(super) fn of(name: String, section: String, text: &str, shelf: Shelf) -> Holding {
        Holding {
            name,
            section,
            disclosure: first_line(text),
            hash: B3Hash::digest(text.as_bytes()),
            carried: shelf.address().is_none().then(|| text.to_owned()),
            shelf,
            package: None,
        }
    }
}

/// The first non-empty line of a document, trimmed of its heading mark:
/// what the author wrote to describe it, and the one line a catalog
/// would show.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .trim_start_matches(['#', ' '])
        .to_owned()
}
