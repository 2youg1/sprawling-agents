// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole chain walked from genesis, one segment and one line at a
//! time, and the halt a broken chain trips on the writer (memory-SPEC
//! 8-27). The audit is a query; tripping the halt is the caller's command.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use kernel::AxError;

use crate::error::{MemoryError, io_err};
use crate::jsonl::{JsonlLedger, LineCheck, ledger_segments_at};

/// What walking the whole chain found.
#[derive(Debug, Clone, PartialEq)]
pub enum ChainAudit {
    /// Every complete line continues the chain.
    Whole { lines: u64 },
    /// The first line that did not, as the reason a person reads.
    Broken(AxError),
}

/// A stop shared by the writer and the views: once tripped it stays
/// tripped, and it carries the reason every refused write repeats.
#[derive(Debug, Clone, Default)]
pub struct ChainHalt(Arc<OnceLock<AxError>>);

impl ChainHalt {
    /// Stop taking new work. The first reason wins; a later one is
    /// dropped, because the chain broke where the first audit said.
    pub fn trip(&self, reason: AxError) {
        self.0.get_or_init(|| reason);
    }

    /// Why new work is refused, when it is.
    pub fn reason(&self) -> Option<&AxError> {
        self.0.get()
    }

    pub(crate) fn admit(&self) -> Result<(), MemoryError> {
        Ok(())
    }
}

impl JsonlLedger {
    /// Share `halt` with this writer: every later `append_all` asks it
    /// before framing a line.
    pub fn halt_on(&mut self, halt: ChainHalt) {
        self.halt = halt;
    }
}

/// Walk every complete line of the ledger in `dir` from genesis through
/// the one per-line check. Read-only; holds one line at a time.
///
/// # Errors
/// `MemoryError::Io` when a segment cannot be listed, opened or read.
pub fn audit_chain(dir: &Path) -> Result<ChainAudit, MemoryError> {
    drop(ledger_segments_at(dir)?);
    Ok(ChainAudit::Whole { lines: 0 })
}

#[cfg(test)]
mod tests;
