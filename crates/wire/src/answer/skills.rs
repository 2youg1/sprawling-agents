// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one building can do, by the shelf each capability sits on.
//!
//! Three shelves in one answer because a reader needs them together to
//! make sense of any: the city's library is the stock any building may
//! admit, the building's own shelf is what only it keeps, and the
//! external shelves are directories the city mounts read-only from
//! elsewhere on this machine. A line is the same seven facts on either
//! shelf, so there is one row type and the shelf is what differs.

use kernel::{Address, B3Hash, RunId};
use serde::{Deserialize, Serialize};

/// Which shelf a holding sits on. City shelves are CAS indexes rather
/// than document directories, so the city arms identify a scope, not a
/// file to pass to `Query::Document`. A building arm names that building;
/// an external arm names the mounted shelf and relative package path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SkillShelf {
    /// The city's central stock, under the reserved prefix.
    Library,
    /// This building's own shelf, inside the building.
    Building(Address),
    /// A directory outside the city that the city's configuration
    /// mounts read-only: which shelf of that list this is, and the
    /// document's path from the shelf's root.
    External { index: u32, path: String },
}

/// One shelved skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillLine {
    pub name: String,
    /// The section the shelf files it under. Empty for a skill on an
    /// external shelf: that tree is another harness's, it files skills
    /// one directory per skill with no section above them, and a
    /// section invented here would be this city's guess about a layout
    /// it does not own.
    pub section: String,
    /// Which shelf, and where the document is on it.
    pub shelf: SkillShelf,
    /// The first non-empty line of the document, which is what its
    /// author wrote to describe it.
    pub disclosure: String,
    /// What the whole document hashed to when this scan read it.
    pub hash: B3Hash,
    /// Whether this building's reading room admits it. A skill on the
    /// shelves that the room does not admit costs a run nothing and is
    /// still worth seeing: it is the list a person edits.
    pub admitted: bool,
    /// The runs that were frozen with this exact hash pinned, newest
    /// last. Empty means nothing has used it, which is a different
    /// fact from nothing having recorded it - every run records its
    /// pins, so an empty list is an answer.
    pub pinned_by: Vec<RunId>,
}

/// The shelves one building reads from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillsAnswer {
    pub building: Address,
    /// Every shelf, city stock first, then this building's own, then the
    /// external shelves in the order the configuration lists them;
    /// within a shelf, section then name order. [`SkillShelf`] says
    /// which shelf a row came from; the scan that fills this list owns
    /// the order, and nothing here re-sorts it.
    pub skills: Vec<SkillLine>,
    /// Names this building's reading room admits that no shelf holds.
    /// Shown to the person who wrote the list, since only they can fix
    /// it.
    pub missing: Vec<String>,
    /// The city layer's ordered `[skills] shelves` entries as written,
    /// including mounts that currently hold no skills. Older answers omit it.
    #[serde(default)]
    pub external_shelves: Vec<String>,
}
