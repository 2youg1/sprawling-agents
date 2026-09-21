// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ledger's side index: seq to (segment, byte
//! offset), so one line costs an open plus a seek instead of a scan.
//!
//! Everything here is disposable. The maps live in memory for the life
//! of the process and are rebuilt from the segments whenever they are
//! opened; there is no persisted copy to believe and no stamp to
//! compare, so no doubt has to be resolved in silence.

mod fold;
mod ledger;
mod reader;

pub use ledger::LedgerIndex;
pub use ledger::Refreshed;
pub use reader::LineReader;
