// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a start begins folding: after a snapshot whose line still sits
//! at its seq, or from genesis with the reason it could not resume
//! (`crates/storage/spec/Snapshot/Start.lean` §8-28).

use std::path::Path;

use kernel::Seq;

use super::{ChainSnapshot, SnapshotFit, StoredSnapshot, read_snapshot};
use crate::error::{StorageError, io_err};
use crate::jsonl::{claimed_seq, complete_lines, ledger_segments_at, segment_first_seq};
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
        StoredSnapshot::Present(snapshot) => return tail_through(vfs, ledger_dir, snapshot),
    };
    Ok(SnapshotStart::Whole(because))
}

/// Where folding goes on after `snapshot`, which the caller has already
/// read and accepted: the lines after its seq when its line still sits
/// there, or genesis with the reason it does not.
///
/// # Errors
/// `StorageError::Io` when a segment cannot be read.
pub fn tail_after(
    ledger_dir: &Path,
    snapshot: ChainSnapshot,
) -> Result<SnapshotStart, StorageError> {
    tail_through(&RealFs::new(), ledger_dir, snapshot)
}

fn tail_through(
    vfs: &dyn Vfs,
    ledger_dir: &Path,
    snapshot: ChainSnapshot,
) -> Result<SnapshotStart, StorageError> {
    let because = match lines_from_cut(vfs, ledger_dir, snapshot.seq())? {
        None => WholeFold::Missing,
        Some(Cut { line_at_seq, tail }) => match snapshot.fit(&line_at_seq) {
            SnapshotFit::Fits => return Ok(SnapshotStart::Resume { snapshot, tail }),
            SnapshotFit::Stale => WholeFold::Stale,
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
/// `seq`. Within that segment the line is found from the end, and by
/// counting from the start when the end cannot say (`crates/storage/spec/Snapshot/Start.lean` §8-28).
fn lines_from_cut(vfs: &dyn Vfs, dir: &Path, seq: Seq) -> Result<Option<Cut>, StorageError> {
    let segments = ledger_segments_at(dir)?;
    let Some((at, first)) = segments.iter().enumerate().rev().find_map(|(at, segment)| {
        first_seq_of(segment)
            .filter(|first| *first <= seq)
            .map(|first| (at, first))
    }) else {
        return Ok(None);
    };
    let mut onward = segments.iter().skip(at);
    let Some(holding) = onward.next() else {
        return Ok(None);
    };
    let bytes = vfs.read(holding).map_err(io_err("read segment", holding))?;
    let Some(mut cut) = from_the_end(&bytes, seq).or_else(|| from_the_start(&bytes, first, seq))
    else {
        return Ok(None);
    };
    for segment in onward {
        let bytes = vfs.read(segment).map_err(io_err("read segment", segment))?;
        cut.tail.extend(
            complete_lines(&bytes)
                .0
                .into_iter()
                .filter(|line| !line.is_empty())
                .map(<[u8]>::to_vec),
        );
    }
    Ok(Some(cut))
}

/// The line at `seq` in one segment's `bytes`, and its complete lines
/// after it, found from the end: the last complete line names its own
/// seq, so the line at `seq` is that many lines back, and only the bytes
/// after it are scanned. `None` when the last line names no seq, names
/// one before `seq`, or the segment holds fewer lines than that.
fn from_the_end(bytes: &[u8], seq: Seq) -> Option<Cut> {
    let end = bytes.iter().rposition(|byte| *byte == b'\n')?;
    let mut lines = bytes
        .get(..end)?
        .rsplit(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty());
    let newest = lines.clone().next()?;
    let back = claimed_seq(newest)?.value().checked_sub(seq.value())?;
    let back = usize::try_from(back).ok()?;
    let mut tail: Vec<Vec<u8>> = lines.by_ref().take(back).map(<[u8]>::to_vec).collect();
    if tail.len() != back {
        return None;
    }
    let line_at_seq = lines.next()?.to_vec();
    tail.reverse();
    Some(Cut { line_at_seq, tail })
}

/// The line at `seq` in one segment's `bytes`, whose first line is
/// `first`, and its complete lines after it, found by counting from the
/// segment's start; `None` when no line sits at `seq`.
fn from_the_start(bytes: &[u8], first: Seq, seq: Seq) -> Option<Cut> {
    // Past the address space the line cannot be in memory either.
    let skip = usize::try_from(seq.value().checked_sub(first.value())?).ok()?;
    let mut lines = complete_lines(bytes)
        .0
        .into_iter()
        .filter(|line| !line.is_empty())
        .skip(skip);
    let line_at_seq = lines.next()?.to_vec();
    let tail = lines.map(<[u8]>::to_vec).collect();
    Some(Cut { line_at_seq, tail })
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
