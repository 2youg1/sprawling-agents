// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The plan as a tree: what the nodes are, what each one is worth, what
//! may be started now, and the two ways a held node is put down.
//!
//! `crate::spine` owns the document — how a row is written and read.
//! This module owns the structure that document describes, and the two
//! are separate because the grammar of a line and the arithmetic of a
//! plan fail in different ways: a mistyped row is repaired by editing
//! it, while a cycle in the dependencies is repaired by rethinking the
//! work.
//!
//! **Weight is conserved because [`crate::share`] is the only way to
//! hold any.** A node's share is divided out of its parent's when the
//! tree is built, so the total is 1 whatever the plan turns into and no
//! version number is needed for the denominator. What grows when a
//! branch is split is the number of leaves, not the total; that is why
//! the interface shows both — a percentage says how much is done and a
//! leaf count says how much was found.
//!
//! **Progress is counted on leaves only.** A branch has no work of its
//! own: its children are its work, and counting both would count the
//! same effort twice. A branch that says `Done` while a child does not
//! is refused where the tree is built, so the status column of a branch
//! is a summary a reader can trust rather than a second opinion.

use std::collections::BTreeMap;

use crate::node_id::NodeId;
use crate::share::WHOLE_PPB;

mod blocking;
mod node;
mod share;
mod tree;

pub use node::{Held, PlanExit, PlanNode, StopCause};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanTree {
    pub(crate) nodes: BTreeMap<NodeId, PlanNode>,
}

/// The whole plan in billionths, re-exported where progress is read so a
/// renderer does not have to know which module the constant lives in.
pub const PLAN_WHOLE_PPB: u64 = WHOLE_PPB;
