// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The files under one address whose name holds a text
//! (`crates/wire/spec/Answer/Find.lean` §8-82).
//!
//! A page that wants one name cannot ask `Listing` level by level: a
//! deep tree is dozens of round trips before it can say "none", and a
//! directory the page never asked is one it cannot speak for. The city
//! walks the tree once and says whether that walk reached the end.

use kernel::Address;
use serde::{Deserialize, Serialize};

/// How many files one answer lists: a bound on the answer's size on
/// the wire, and as many as a list a person picks one file from needs.
pub const FIND_MAX: usize = 50;

/// How many entries one walk looks at before it stops: the cost of one
/// question, paid on every key the person types.
pub const FIND_WALK_MAX: usize = 20_000;

/// Whether the walk looked at everything under the address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Walked {
    /// Every entry was looked at: a file not listed has no such name.
    Whole,
    /// The walk stopped at [`FIND_MAX`] files or [`FIND_WALK_MAX`]
    /// entries with the tree not finished, so more may match.
    Cut,
}

/// The files found, shallow first and in name order within one level,
/// each relative to `under`. `under` and `text` echo the question,
/// which the wire carries no id for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FindAnswer {
    pub under: Address,
    pub text: String,
    pub paths: Vec<Address>,
    pub walked: Walked,
}
