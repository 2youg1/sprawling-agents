// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The system prompt one run was frozen with, and the general read of
//! the content store it is recovered through.
//!
//! Both shapes live here because they are one question asked at two
//! grains: a prefix answer is four store objects named by the record
//! that froze them, and a content answer is one store object named by
//! whoever holds its locator. A page that can draw the first can draw
//! the second with no second vocabulary for "bytes this city kept".

use kernel::{Address, B3Hash, Locator, RunId};
use serde::{Deserialize, Serialize};

/// The four slots of a frozen prefix, in the order they are sent.
///
/// Exhaustive and closed: the order is the cache economics, and a
/// spelling a page had to match against a free string would let a
/// segment arrive under a name nothing draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrefixSlot {
    City,
    Building,
    Resident,
    Run,
}

/// One document a segment was assembled from, as the assembler saw it.
///
/// `dropped` is what the per-slot budget cut off the end of that
/// document, so a reader can tell a segment that carries a whole file
/// from one that carries the first half of it. Zero is the ordinary
/// case and is still reported, because "nothing was cut" is a fact and
/// an absent field is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixSource {
    pub addr: Address,
    pub kept: u64,
    pub dropped: u64,
}

/// One frozen segment, as a reader needs it.
///
/// `bytes` is what the record says the segment measured and `text` is
/// what the store still holds, so the two disagreeing is itself the
/// finding: `stored` false means the object is gone and the text is
/// empty because nothing could be read, not because the segment was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixSegment {
    pub slot: PrefixSlot,
    pub hash: B3Hash,
    pub bytes: u64,
    pub text: String,
    pub stored: bool,
    pub sources: Vec<PrefixSource>,
}

/// What one run was told before it said anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrefixAnswer {
    pub run: RunId,
    /// City, building, resident, run - always four, in that order.
    pub segments: Vec<PrefixSegment>,
}

/// One object of the content store, cut to what travels.
///
/// The same bound and the same text judgement `Document` puts on a file
/// in the tree, for the same reasons: bytes with a NUL in the head are
/// not text, and showing them as text would show a reader something the
/// object does not say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ContentAnswer {
    /// The locator asked for, handed back: the wire carries no request
    /// id, so a page matches an answer to its question by content.
    pub locator: Locator,
    pub text: String,
    pub bytes: u64,
    pub truncated: bool,
    pub binary: bool,
}
