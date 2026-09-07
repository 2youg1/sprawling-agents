// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The leaf: which face of a building opens first.

use channels::{Address, BuildingAnswer};

/// Which of a building's faces is showing. The documents are named by the
/// files themselves, so the only variants this type spells are the ones
/// that are not a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Leaf {
    /// The plan tree, by state. First because it is what the building is
    /// for: a person opening one wants to know what it is doing, and
    /// `Roadmap.md` read as prose is the same facts in the shape a
    /// parser wanted rather than the shape a reader wants.
    Plan,
    /// One document, by file name.
    Doc(String),
    /// The archive index.
    Archive,
    /// One room, and what waits in it.
    Room(String),
    /// What this building's runs may reach: the only face on this page
    /// a person writes through, and the only one no agent can.
    Reach,
}

/// The address of one room of this building.
///
/// The directory tree is the space (glossary, "Floor / Room"), so a room's
/// address is its building's plus the directory's own name. The city keeps
/// the authority on what an address may contain; this composes and lets
/// `Address::parse` refuse, rather than deciding for itself what a legal
/// room name is.
#[must_use]
pub fn room_addr(building: &Address, room: &str) -> Option<Address> {
    Address::parse(&format!("{}/{room}", building.as_str())).ok()
}

/// The first thing to show for a building: its plan, unless it has none.
///
/// The board rather than the file, when the plan parses. Both say the
/// same thing; only one of them says which nodes a person could hand out
/// right now.
#[must_use]
pub fn opening_leaf(answer: &BuildingAnswer) -> Leaf {
    if !answer.plan.is_empty() {
        return Leaf::Plan;
    }
    answer
        .docs
        .first()
        .map_or(Leaf::Archive, |doc| Leaf::Doc(doc.name.clone()))
}
