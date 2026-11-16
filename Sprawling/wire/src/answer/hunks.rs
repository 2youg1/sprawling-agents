// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One file's patch text between two checkpoints.
//!
//! Its own file for the reason `Query::Hunks` is its own frame: a change
//! list costs what the number of changed files costs and this costs what
//! one file costs, so the two answers are kept apart everywhere they
//! appear.

use kernel::GitOid;
use serde::{Deserialize, Serialize};

/// One file's patch text between two checkpoints, and what could not be
/// shown.
///
/// Both ends travel back with the answer because they are what makes it
/// cacheable: two commit ids never change, so whoever asked may keep
/// this for as long as they like.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HunksAnswer {
    pub oid_a: GitOid,
    pub oid_b: GitOid,
    pub path: String,
    pub lines: Vec<PatchLine>,
    /// Lines that matched a credential shape. They are named and not
    /// echoed, for the reason a staged blob's scan gives about its own
    /// hits: printing the bytes to prove a leak is the leak.
    pub withheld: Vec<Withheld>,
}

/// One line of patch text, numbered from the top of the patch so a
/// withheld line and the lines around it read as one list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PatchLine {
    pub number: u32,
    pub text: String,
}

/// One line that was not echoed, and what matched it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Withheld {
    pub number: u32,
    /// A provider's own key shape by name, or the entropy judgement when
    /// no shape claimed it. Never the bytes.
    pub reason: String,
}
