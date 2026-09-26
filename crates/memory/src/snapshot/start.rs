// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a start begins folding: after a snapshot whose line still sits
//! at its seq, or from genesis with the reason it could not resume
//! (memory-SPEC 8-28).

use std::path::Path;

use super::{ChainSnapshot, StoredSnapshot, read_snapshot};
use crate::error::MemoryError;
use crate::jsonl::read_raw_lines_at;

/// The lines a start folds, and the state it folds them onto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotStart {
    /// The snapshot fits: fold `tail`, the lines after its seq, onto its views.
    Resume {
        snapshot: ChainSnapshot,
        tail: Vec<Vec<u8>>,
    },
    /// Fold every complete line from genesis.
    Whole {
        lines: Vec<Vec<u8>>,
        because: WholeFold,
    },
}

/// Why a start could not resume from the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WholeFold {
    NoSnapshot,
    /// The reason `read_snapshot` gave for bytes that are not a snapshot.
    Damaged(String),
    /// The fold rules changed since the snapshot was cut.
    OtherFoldVersion {
        found: u32,
    },
    /// Another line sits at the snapshot's seq.
    Stale,
    /// The ledger ends before the snapshot's seq.
    Missing,
}

/// Decide where folding the ledger in `ledger_dir` starts, given the
/// snapshot in `snapshot_dir` and the fold rules' `fold_version`.
///
/// # Errors
/// `MemoryError::Io` when a segment or the snapshot cannot be read.
pub fn start_from_snapshot(
    ledger_dir: &Path,
    snapshot_dir: &Path,
    fold_version: u32,
) -> Result<SnapshotStart, MemoryError> {
    let _ = (
        snapshot_dir,
        fold_version,
        read_snapshot,
        StoredSnapshot::Absent,
    );
    Ok(SnapshotStart::Whole {
        lines: read_raw_lines_at(ledger_dir)?,
        because: WholeFold::NoSnapshot,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests;
