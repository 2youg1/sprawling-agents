// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One node, one working tree.
//!
//! Concurrency inside a building is not held apart by discipline but by
//! the filesystem: each node works in its own checkout, so two agents
//! cannot see each other's half-finished state, and what comes back
//! comes back through the PR flow.
//!
//! The trees are git worktrees of the repository `crate::checkpoint`
//! already keeps, so objects are shared and only the working files are
//! duplicated. A tree is refused before it is created, never halfway
//! through: the size of the tree to be copied is measured against
//! `WORKTREE_MAX_BYTES` first, and the refusal states both numbers.
//!
//! Copy-on-write cloning (reflink) is the cheaper path on filesystems
//! that offer it, and this module does not attempt it: there is no CoW
//! interface without unsafe FFI or a new dependency, so today every tree
//! is a full checkout under the ceiling. That is the fallback arm of the
//! design, stated as the current state rather than as the design.

mod lease;
mod name;
mod trees;

pub use lease::WorktreeLease;
pub use name::WorktreeName;
pub use trees::{PlannedMerge, Worktrees};
