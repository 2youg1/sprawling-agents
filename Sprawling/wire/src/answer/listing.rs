// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One directory of the city, as a page walks the tree.
//!
//! The tree is the product: a building is a directory, a room is a
//! directory inside it, and what a resident wrote lives beside the
//! transcript of the run that wrote it. `BuildingView` answers with the
//! documents at a building's root and the names of its rooms, and
//! nothing on the wire could open a room - so the one design the
//! interface exists to show was the one thing it could not draw.
//!
//! One level per question. A page opens directories as a person does,
//! and an answer that carried the whole subtree would pay for the
//! ledger segments and the content store every time somebody looked at
//! a room.

use kernel::Address;
use serde::{Deserialize, Serialize};

/// What one entry is. A directory has no size worth stating: the size
/// of a directory is a question about everything under it, and this
/// answer deliberately does not walk that far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum EntryKind {
    Directory,
    File { bytes: u64 },
}

/// One name inside a directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
}

/// One directory, one level deep: directories first, then files, each
/// group in name order, so a tree drawn from it reads the same way on
/// every machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ListingAnswer {
    /// The directory listed; `None` is the city root.
    pub at: Option<Address>,
    pub entries: Vec<Entry>,
}
