// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a start begins folding: after a snapshot whose line still sits
//! at its seq, or from genesis with the reason it could not resume
//! (storage-SPEC 8-28).

use std::path::Path;

use kernel::Seq;

use super::{ChainSnapshot, SnapshotFit, StoredSnapshot, read_snapshot};
use crate::error::{StorageError, io_err};
use crate::jsonl::{complete_lines, ledger_segments_at, segment_first_seq};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

/// Where a start folds from: the state after a snapshot and the lines
/// past it, or genesis and the reason no snapshot served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotStart {
    /// The snapshot fits: fold `tail`, the lines after its seq, onto its views.
    Resume {
        snapshot: ChainSnapshot,
        tail: Vec<Vec<u8>>,
    },
    /// Fold from genesis. The lines are not carried: the caller streams
    /// them a segment at a time rather than holding the whole ledger.
    Whole(WholeFold),
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
/// Only the segment holding the snapshot's seq and the segments after
/// it are read; the whole chain is `prove_chain`'s to walk.
///
/// # Errors
/// `StorageError::Io` when a segment or the snapshot cannot be read.
pub fn start_from_snapshot(
    ledger_dir: &Path,
    snapshot_dir: &Path,
    fold_version: u32,
) -> Result<SnapshotStart, StorageError> {
    start_through(&RealFs::new(), ledger_dir, snapshot_dir, fold_version)
}

/// [`start_from_snapshot`], reading segments through `vfs`: the seam a
/// test counts what an opening reads through.
pub(crate) fn start_through(
    vfs: &dyn Vfs,
    ledger_dir: &Path,
    snapshot_dir: &Path,
    fold_version: u32,
) -> Result<SnapshotStart, StorageError> {
    let because = match read_snapshot(snapshot_dir)? {
        StoredSnapshot::Absent => WholeFold::NoSnapshot,
        StoredSnapshot::Damaged(reason) => WholeFold::Damaged(reason),
        StoredSnapshot::Present(snapshot) if snapshot.fold_version() != fold_version => {
            WholeFold::OtherFoldVersion {
                found: snapshot.fold_version(),
            }
        }
        StoredSnapshot::Present(snapshot) => match lines_from_cut(vfs, ledger_dir, snapshot.seq())?
        {
            None => WholeFold::Missing,
            Some(Cut { line_at_seq, tail }) => match snapshot.fit(&line_at_seq) {
                SnapshotFit::Fits => return Ok(SnapshotStart::Resume { snapshot, tail }),
                SnapshotFit::Stale => WholeFold::Stale,
            },
        },
    };
    Ok(SnapshotStart::Whole(because))
}

/// The line a snapshot was cut at, and the lines after it.
struct Cut {
    line_at_seq: Vec<u8>,
    tail: Vec<Vec<u8>>,
}

/// The line at `seq` and every complete line after it, read from the
/// segment whose name claims `seq` onward; `None` when no line sits at
/// `seq`.
fn lines_from_cut(vfs: &dyn Vfs, dir: &Path, seq: Seq) -> Result<Option<Cut>, StorageError> {
    let segments = ledger_segments_at(dir)?;
    let Some((at, first)) = segments.iter().enumerate().rev().find_map(|(at, segment)| {
        first_seq_of(segment)
            .filter(|first| *first <= seq)
            .map(|first| (at, first))
    }) else {
        return Ok(None);
    };
    // Past the address space the line cannot be in memory either: skip all.
    let mut skip = usize::try_from(seq.value().saturating_sub(first.value())).unwrap_or(usize::MAX);
    let (mut line_at_seq, mut tail) = (None, Vec::new());
    for segment in segments.iter().skip(at) {
        let bytes = vfs.read(segment).map_err(io_err("read segment", segment))?;
        for line in complete_lines(&bytes)
            .0
            .into_iter()
            .filter(|line| !line.is_empty())
        {
            match (skip.checked_sub(1), &line_at_seq) {
                (Some(left), _) => skip = left,
                (None, None) => line_at_seq = Some(line.to_vec()),
                (None, Some(_)) => tail.push(line.to_vec()),
            }
        }
    }
    Ok(line_at_seq.map(|line_at_seq| Cut { line_at_seq, tail }))
}

fn first_seq_of(segment: &Path) -> Option<Seq> {
    segment
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(segment_first_seq)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    reason = "test code"
)]
mod tests;
