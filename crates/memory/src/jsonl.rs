// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Durable Ledger adapter: jsonl segments, tail-truncation recovery,
//! direction-aware version refusal, group commit.
//!
//! Contract owned here:
//! - bytes come from `EventRecord::canonical_line` verbatim plus `\n`;
//!   this module owns seq/prev assignment and the fsync schedule, nothing
//!   about the byte shape.
//! - `append_all` is one durability barrier per wave: `Ok` means every
//!   line is written and synced; on `Err` the in-memory state has not
//!   advanced and torn bytes are the next open's tail recovery.
//! - open never rewrites while browsing: the version probe fires before
//!   any repair; only the last segment is ever truncated, and only at a
//!   torn boundary.
//! - time is a parameter (`now` feeds the `log_truncated` record); this
//!   module never samples a clock (determinism rule 2).
//!
//! The `Vfs` seam this reaches disk through is crate-internal on
//! purpose (`crate::vfs`): std fs and FaultFs are its two adapters, and
//! it never enters a public signature — `JsonlLedger` hides it behind
//! `Box<dyn Vfs>`.

mod append;
mod ledger;
mod open;

pub use append::{ledger_segments_at, read_raw_lines_at};
pub(crate) use ledger::is_segment;
pub use ledger::{JsonlLedger, OpenReport, TailTruncation, WriteObserver};
