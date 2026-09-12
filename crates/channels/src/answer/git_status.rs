// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What is uncommitted in one building, and the last fence behind it.
//!
//! The rows are [`FileChange`], the same shape `Changes` answers with:
//! "which files moved and how much" is one question whether the far end
//! is a checkpoint or the disk, and a second row type would be a second
//! answer to it.

use kernel::{Address, FileChange};
use serde::{Deserialize, Serialize};

use super::CommitAnswer;

/// How far a branch has drifted from the upstream it tracks.
///
/// Absent as a whole rather than as two zeroes when the branch tracks
/// nothing: a branch with no upstream is not a branch level with one,
/// and a page that read `0/0` for both would tell a person their work
/// is pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Drift {
    pub ahead: u64,
    pub behind: u64,
}

/// The working tree of one building, read at the moment of asking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GitStatusAnswer {
    pub building: Address,
    /// The branch checked out, or absent on a detached head - which is
    /// a state a person can be in and one a name cannot describe.
    pub branch: Option<String>,
    /// Absent when the branch tracks no upstream.
    pub drift: Option<Drift>,
    /// Files under this building that differ from the last commit,
    /// untracked ones included, in path order.
    pub files: Vec<FileChange>,
    /// The newest commit this city fenced at this building, from the
    /// history rather than from git, for the reason `Query::Commit`
    /// gives. Absent when the city has fenced nothing here.
    pub checkpoint: Option<CommitAnswer>,
}
