// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod append;
mod barrier;
mod boundary;
mod first_line;
mod ledger;
mod open;
mod reading;
mod tail;
mod unwind;
mod verify;

pub(crate) use first_line::{FIRST_WINDOW_BYTES, first_line};
#[cfg(test)]
pub(crate) use ledger::segment_file_name;
pub use ledger::{JsonlLedger, LedgerHold, OpenReport, TailTruncation, WriteObserver};
pub(crate) use ledger::{complete_lines, records_end, segment_first_seq, segment_names};
pub(crate) use reading::SCAN_WINDOW_BYTES;
#[cfg(test)]
pub(crate) use reading::measure as measure_scan;
pub use reading::{ledger_segments_at, read_raw_lines_at};
pub use tail::{TailLine, TailLines};
pub(crate) use verify::claimed_seq;
pub use verify::{CheckedLine, LineCheck, LineFault, read_line};
