// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one building can do, by the shelf each capability sits on.
//!
//! Two shelves in one answer because a reader needs both to make sense
//! of either: the city's library is the stock any building may admit,
//! and the building's own shelf is what only it keeps. A line is the
//! same six facts on either shelf, so there is one row type and the
//! shelf is what differs.

use kernel::{Address, B3Hash, RunId};
use serde::{Deserialize, Serialize};

/// Which shelf a holding sits on.
///
/// Named rather than implied by which list it arrived in, so a page
/// that shows both shelves in one list can still say where a skill
/// came from - which is the question a person asks when two shelves
/// hold the same name and the nearer one wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SkillShelf {
    /// The city's central stock, under the reserved prefix.
    Library,
    /// This building's own shelf, inside the building.
    Building,
}

/// One shelved skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillLine {
    pub name: String,
    pub section: String,
    pub shelf: SkillShelf,
    /// Where the document sits, as the city spells it - which is what
    /// `Query::Document` takes, so opening a skill needs no second
    /// question about where its file is.
    pub at: Address,
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
    /// Both shelves, city stock first, then this building's own; within
    /// a shelf, section then name order.
    pub skills: Vec<SkillLine>,
    /// Names this building's reading room admits that no shelf holds.
    /// Shown to the person who wrote the list, since only they can fix
    /// it.
    pub missing: Vec<String>,
}
