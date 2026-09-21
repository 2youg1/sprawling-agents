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

use kernel::{Address, B3Hash};

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
}
