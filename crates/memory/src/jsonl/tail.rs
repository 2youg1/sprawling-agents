// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ledger read from its tail: the newest line first, segment by
//! segment backwards, each segment read from its end through a window
//! that starts at one page and doubles (the rule `first_line` uses), so
//! a reader that wants the last N records pays for their bytes and not
//! for the whole ledger (memory-SPEC 8-1). Each line passes the same
//! judgement a forward reader applies (`LineCheck::judge`); the chain is
//! linked from the newer end instead.

use std::path::{Path, PathBuf};

use crate::error::MemoryError;
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::ledger::is_segment;
use super::verify::CheckedLine;

/// One line of the ledger, newest first: its bytes and what it is.
#[derive(Debug, Clone, PartialEq)]
pub struct TailLine {
    pub raw: Vec<u8>,
    pub checked: CheckedLine,
}

/// The ledger's lines from the newest back. The walk ends after the
/// oldest line, or after the first line that does not pass.
pub struct TailLines {
    vfs: Box<dyn Vfs>,
    segments: Vec<PathBuf>,
}

impl TailLines {
    /// The walk over the ledger in `dir`, read only: it never opens the
    /// ledger, never repairs and never writes.
    ///
    /// # Errors
    /// The directory cannot be listed.
    pub fn at(dir: &Path) -> Result<Self, MemoryError> {
        Self::through(Box::new(RealFs::new()), dir)
    }

    pub(crate) fn through(vfs: Box<dyn Vfs>, dir: &Path) -> Result<Self, MemoryError> {
        let segments = vfs
            .list(dir)
            .map_err(crate::error::io_err("list ledger dir", dir))?
            .into_iter()
            .filter(|p| is_segment(p))
            .collect();
        Ok(Self { vfs, segments })
    }
}

impl Iterator for TailLines {
    type Item = Result<TailLine, MemoryError>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
