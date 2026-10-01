// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `Views::rebuild` throws the whole thing
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

pub mod answered;
pub mod answering;
pub mod archives;
mod asking;
pub mod building_page;
pub mod city;
pub mod commits;
pub mod cost_of;
pub mod document;
pub mod evidence;
pub mod git_status;
pub mod governance;
#[cfg(test)]
mod governance_tests;
pub mod hearing;
pub mod holding;
pub mod hunks;
pub mod lines;
pub mod listing;
pub mod mcp_health;
pub mod prefix;
pub mod prepared;
pub mod published;
pub mod rounds;
pub mod served;
mod sessions;
pub mod skills;
pub mod snapshot;
#[cfg(test)]
mod standing_tests;
#[cfg(test)]
mod tests;
pub mod toolkits;

pub use asking::ask;
pub use governance::Governance;
pub use holding::Views;
pub use lines::pursued;
pub use published::{Published, answer_outside_the_lock};
pub use rounds::turns;
