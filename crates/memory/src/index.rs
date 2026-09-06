// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ledger's side index: seq to (segment, byte
//! offset), so one line costs an open plus a seek instead of a scan.
//!
//! Everything here is disposable. The on-disk cache is believed only
//! when its stamp matches what the directory looks like right now; any
//! doubt — parse failure, byte-count drift, checksum mismatch — rebuilds
//! in silence rather than reporting. A side artifact that lies is worse
//! than one that is missing, so this module never lets a stale cache
//! survive a comparison it cannot pass.

mod cache;
mod ledger;
mod reader;

pub use ledger::LedgerIndex;
pub use reader::LineReader;
