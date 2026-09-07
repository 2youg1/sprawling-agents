// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The root: what the client believes, and which region shows it (web-SPEC.md section 8-1).
//!
//! Index only: `snapshot` owns the fold, `rows` owns the row types and namings.

mod fold;
mod reading;
mod rows;
mod snapshot;
#[cfg(test)]
mod tests;

pub use rows::{ProviderHealth, RunRow, Usage};
pub use snapshot::{Backfill, Snapshot, rebuild};
#[cfg(test)]
pub(crate) use tests::{record, returned_for_test, seated};
