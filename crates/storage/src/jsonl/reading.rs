// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The read-only faces for replay, folds and fixtures: they never open
//! the ledger, never repair, never write — replay must not mutate what it
//! verifies (`crates/runtime/Spec.lean` §8-1). Complete lines only; a torn tail
//! byte-run is not a line and is left for `open` to judge.
//!
//! Specified by `crates/storage/spec/Jsonl.lean` §8-1.

use std::path::{Path, PathBuf};

use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::ledger::{complete_lines, is_segment};

/// The bytes of one ledger segment, read without opening the ledger.
pub(crate) struct SegmentBytes {
    bytes: Vec<u8>,
}

impl SegmentBytes {
    /// The segment's complete, non-empty lines, in file order.
    pub(crate) fn lines(&self) -> impl Iterator<Item = &[u8]> {
        complete_lines(&self.bytes)
            .0
            .into_iter()
            .filter(|line| !line.is_empty())
    }
}

/// Reads one segment `ledger_segments_at` named.
pub(crate) fn read_segment(segment: &Path) -> Result<SegmentBytes, StorageError> {
    RealFs::new()
        .read(segment)
        .map(|bytes| {
            #[cfg(test)]
            measure(bytes.capacity(), bytes.len());
            SegmentBytes { bytes }
        })
        .map_err(io_err("read segment", segment))
}

/// Every complete line of the ledger in `dir`, copied out. For callers
/// that keep the lines themselves (fork, replay, fixtures); a fold that
/// wants only what the lines say walks `LedgerIndex::folding` instead.
pub fn read_raw_lines_at(dir: &Path) -> Result<Vec<Vec<u8>>, StorageError> {
    let mut out = Vec::new();
    for segment in ledger_segments_at(dir)? {
        out.extend(read_segment(&segment)?.lines().map(<[u8]>::to_vec));
    }
    Ok(out)
}

/// The ledger segments in `dir`, in the order they must be read.
///
/// An empty result means the directory holds no ledger at all, which is a
/// different fact from a ledger that holds no events, and only this face
/// can tell them apart: `read_raw_lines_at` answers `Ok([])` to both. The
/// caller that has to tell them apart is the one that took the path from
/// a person - `sprawling replay` - and it asks here so that the segment
/// naming rule is never spelled a second time somewhere else.
pub fn ledger_segments_at(dir: &Path) -> Result<Vec<PathBuf>, StorageError> {
    let vfs = RealFs::new();
    Ok(vfs
        .list(dir)
        .map_err(io_err("list ledger dir", dir))?
        .into_iter()
        .filter(|p| is_segment(p))
        .collect())
}

#[cfg(test)]
std::thread_local! {
    static MEASURED: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

#[cfg(test)]
pub(super) fn measure(held: usize, copied: usize) {
    MEASURED.with(|measured| {
        let (peak, copies) = measured.get();
        measured.set((peak.max(held), copies.saturating_add(copied)));
    });
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::jsonl::JsonlLedger;
    use kernel::{EventDraft, EventKind, Payload, RunId, TimeMs};

    #[test]
    fn three_read_paths_hold_less_than_the_preallocated_segment() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("ledger");
        let (mut ledger, _) = JsonlLedger::open(&dir, TimeMs::new(0)).unwrap();
        ledger
            .append_all(vec![EventDraft {
                run: RunId::CITY,
                t: TimeMs::new(0),
                who: "city".into(),
                addr: None,
                kind: EventKind::GateChecked,
                data: Payload::empty(),
                ig: false,
            }])
            .unwrap();
        let expected = ledger.read_raw_lines().unwrap();
        let segment = ledger_segments_at(&dir).unwrap().remove(0);
        let total = 2 * 1024 * 1024;
        std::fs::OpenOptions::new()
            .write(true)
            .open(&segment)
            .unwrap()
            .set_len(total)
            .unwrap();
        MEASURED.with(|m| m.set((0, 0)));
        assert_eq!(ledger.read_raw_lines().unwrap(), expected);
        let instance = MEASURED.with(std::cell::Cell::get);
        MEASURED.with(|m| m.set((0, 0)));
        assert_eq!(read_raw_lines_at(&dir).unwrap(), expected);
        let readonly = MEASURED.with(std::cell::Cell::get);
        drop(ledger);
        MEASURED.with(|m| m.set((0, 0)));
        let (reopened, report) = JsonlLedger::open(&dir, TimeMs::new(1)).unwrap();
        assert!(report.recovered.is_none());
        assert_eq!(reopened.position().value(), 1);
        let opening = MEASURED.with(std::cell::Cell::get);
        for (name, (peak, copied)) in [
            ("instance", instance),
            ("readonly", readonly),
            ("open", opening),
        ] {
            eprintln!("{name}: peak={peak} copied_into_scan={copied} segment={total}");
            assert!(
                peak < usize::try_from(total / 8).unwrap(),
                "{name}: retained {peak}"
            );
            assert!(
                copied < usize::try_from(total / 8).unwrap(),
                "{name}: copied {copied}"
            );
        }
    }
}
