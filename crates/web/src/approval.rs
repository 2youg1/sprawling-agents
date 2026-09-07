// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Two lists that share one shape: what waits for a person, and what was discarded.
//!
//! Index only.

mod bin;
mod inbox;
#[cfg(test)]
mod tests;

pub use bin::{BinRow, RecycleBinView, ReturnPath, bin_rows, recycle_bin};
pub use inbox::{ApprovalsView, Cluster, inbox, policy_admits};
