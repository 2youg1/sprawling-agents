// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `rebuild_views` throws the whole thing
//! away and folds the ledger again to get the same bytes. That is
//! ARCHITECTURE.md section 9 shape 7, while `bin::assembly` is an
//! adapter - and a file holding two shapes is what section 9 says a split
//! looks like.
//!
//! **What it deliberately does not hold.** The plans are
//! `crate::plan_view`'s and are read through it; a second parse here
//! would be a second answer to "what is stuck and why", and only one of
//! them would be folding the records that say why. What waits in a room
//! is folded from signal records rather than read off a queue, because a
//! queue answers by being consumed and a view that consumed what it
//! showed would change the thing it reports on.

pub(super) mod answering;
pub(super) mod archives;
pub(super) mod commits;
pub(super) mod cost_of;
pub(super) mod document;
pub(super) mod evidence;
pub(super) mod git_status;
#[cfg(test)]
mod governance_tests;
pub(super) mod hearing;
pub(super) mod holding;
pub(super) mod hunks;
pub(super) mod lines;
pub(super) mod listing;
pub(super) mod mcp_health;
pub(super) mod prefix;
pub(super) mod rounds;
pub(super) mod served;
pub(super) mod skills;
#[cfg(test)]
mod standing_tests;
#[cfg(test)]
mod tests;
pub(super) mod toolkits;

pub(crate) use holding::Views;
pub use holding::ask;
pub(crate) use lines::pursuit_from;
