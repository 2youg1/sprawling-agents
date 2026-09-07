// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The first screen: the box that starts work, and the table its rows land in.
//!
//! Index only.

mod composer;
mod listing;
mod page;
mod plan;
mod tables;
#[cfg(test)]
mod tests;

pub use listing::{SeatRow, counts_said, listing, spent_of};
pub use page::SessionsView;
pub use plan::{DEFAULT_MODE, ENDED_ROWS, Field, Plan, latest_room};
