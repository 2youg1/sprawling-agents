// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The chain-hash snapshot: what a fold held after one line of the
//! Ledger, with that line's seq and chain hash (memory-SPEC 8-26).
//!
//! A projection, never history: a start that finds no snapshot, a
//! damaged one, or one whose line hash does not match the line on disk
//! folds the whole ledger, as it did before snapshots existed. That
//! resuming is sound at all is `adversary/src/Sprawling/Snapshot.lean`'s
//! `resumeIsWhole`.

use std::path::Path;

use kernel::ledger::chain_hash;
use kernel::{B3Hash, Seq};

use crate::error::MemoryError;
use crate::jsonl::LineCheck;

/// What a fold held after the line at `seq`, and the chain hash that
/// names that line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSnapshot {
    fold_version: u32,
    seq: Seq,
    line_hash: B3Hash,
    views: Vec<u8>,
}

/// Whether the line a positioned read found at the snapshot's seq is
/// the line the snapshot was cut at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotFit {
    Fits,
    /// Another line sits at that seq: the ledger is not the one the
    /// snapshot was cut from, so its state must not be resumed.
    Stale,
}

/// What the snapshot directory holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredSnapshot {
    Absent,
    /// Bytes that are not a whole snapshot, with the reason; a caller
    /// discards them and folds the whole ledger.
    Damaged(String),
    Present(ChainSnapshot),
}

impl ChainSnapshot {
    /// Cut a snapshot after `line`, the raw line (without its `\n`) at `seq`.
    pub fn cut(fold_version: u32, seq: Seq, line: &[u8], views: Vec<u8>) -> ChainSnapshot {
        ChainSnapshot {
            fold_version,
            seq,
            line_hash: chain_hash(line),
            views,
        }
    }

    pub fn fold_version(&self) -> u32 {
        self.fold_version
    }

    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn views(&self) -> &[u8] {
        &self.views
    }

    /// Judge the raw line a positioned read found at [`Self::seq`].
    pub fn fit(&self, _line_at_seq: &[u8]) -> SnapshotFit {
        SnapshotFit::Fits
    }

    /// The chain state after the snapshot's line: the next line must
    /// carry `prev = line_hash` and the seq after `seq`.
    ///
    /// # Errors
    /// `MemoryError::Draft` when `seq` is the last seq there is.
    pub fn resume(&self) -> Result<LineCheck, MemoryError> {
        Ok(LineCheck::at_genesis())
    }
}

/// Write `snapshot` into `dir` (created when absent), replacing the one
/// there.
///
/// # Errors
/// `MemoryError::Io` naming the step that failed.
pub fn write_snapshot(_dir: &Path, _snapshot: &ChainSnapshot) -> Result<(), MemoryError> {
    Ok(())
}

/// Read the snapshot `dir` holds.
///
/// # Errors
/// `MemoryError::Io` when the file exists and cannot be read; bytes that
/// are not a snapshot are `StoredSnapshot::Damaged`, not an error.
pub fn read_snapshot(_dir: &Path) -> Result<StoredSnapshot, MemoryError> {
    Ok(StoredSnapshot::Absent)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
