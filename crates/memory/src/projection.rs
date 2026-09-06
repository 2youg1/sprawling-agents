// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The cold view: the questions too big for memory
//! and too slow for a scan — what is in the Recycle Bin, how far each
//! Run got, where to resume after a restart.
//!
//! It is derived, never authoritative. Delete the file and replay the
//! ledger and you get the same view back; that is the property the
//! tests assert, and it is why `apply` skips anything at or below the
//! last applied seq — a caller that cannot remember where it stopped
//! may simply start over.
//!
//! Determinism is asserted on [`Projection::export_canonical`], not on
//! the database file. redb is free to differ byte-for-byte between
//! runs (allocator state, page reuse); the logical content is not.

mod tables;
mod view;

pub use tables::{ProjectionOpenReport, RecycleEntry, RunRow, ViewRebuilt};
pub use view::Projection;
