// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two steps of the unified history that write a tree: going back to
//! a point, and taking one file back from a point.
//!
//! `crates/storage/spec/Worktree/Back.lean` requires that each step appends
//! exactly one record and that an undo is one of them, never a removal;
//! these are those records. The tree is named by its worktree name, as
//! in `worktree_opened`, and never by its path on disk, because a
//! history that carries a path is a fact about one machine.

use serde::{Deserialize, Serialize};

use crate::locator::GitOid;

/// `went_back`: the tree `name` was opened with its branch at `point`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WentBack {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub point: GitOid,
}

/// `file_restored`: `path` in the tree `name` now holds what `point`
/// holds there, or nothing when `point` holds no such file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FileRestored {
    pub name: String,
    /// Relative and `/`-separated, as a git tree spells it, so the same
    /// restore records the same bytes on every machine.
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub point: GitOid,
}
