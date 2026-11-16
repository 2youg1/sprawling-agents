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

use super::ledger::{is_segment, u64_count};

/// The maximum range the forward scanner asks the filesystem for.
pub(crate) const SCAN_WINDOW_BYTES: u64 = 64 * 1024;

/// Where the scan ended physically and where its nonzero bytes ended.
pub(super) struct ScanEnd {
    pub(super) total: u64,
    pub(super) filled: u64,
}

/// Visits complete lines, including empty ones, with their end offsets.
/// A zero run is materialized only when another nonzero byte follows it;
/// an unterminated tail is never visited. Working memory is one window
/// and the longest line, independent of the preallocated zero tail.
pub(super) fn scan(
    vfs: &dyn Vfs,
    path: &Path,
    from: u64,
    mut visit: impl FnMut(&mut Vec<u8>, u64) -> Result<(), StorageError>,
) -> Result<ScanEnd, StorageError> {
    let mut end = ScanEnd {
        total: from,
        filled: from,
    };
    let mut line = Vec::new();
    let mut zeros = 0usize;
    loop {
        let chunk = vfs
            .read_at(path, end.total, SCAN_WINDOW_BYTES)
            .map_err(io_err("read segment", path))?;
        #[cfg(test)]
        measure(chunk.capacity().saturating_add(line.capacity()), 0);
        for byte in &chunk {
            end.total = end.total.checked_add(1).ok_or_else(|| {
                io_err("measure segment", path)(std::io::Error::other("segment offset overflow"))
            })?;
            if *byte == 0 {
                zeros = zeros.checked_add(1).ok_or_else(|| {
                    io_err("measure zero run", path)(std::io::Error::other("zero run overflow"))
                })?;
                continue;
            }
            end.filled = end.total;
            let expanded = line.len().checked_add(zeros).ok_or_else(|| {
                io_err("measure segment line", path)(std::io::Error::other("line length overflow"))
            })?;
            line.resize(expanded, 0);
            #[cfg(test)]
            measure(chunk.capacity().saturating_add(line.capacity()), zeros);
            zeros = 0;
            if *byte == b'\n' {
                visit(&mut line, end.total)?;
                line.clear();
            } else {
                line.push(*byte);
                #[cfg(test)]
                measure(chunk.capacity().saturating_add(line.capacity()), 1);
            }
        }
        if u64_count(chunk.len()).map_err(io_err("measure read window", path))? < SCAN_WINDOW_BYTES
        {
            return Ok(end);
        }
    }
}

/// Reads one segment's complete, nonempty lines without opening a ledger.
pub(crate) fn read_segment(segment: &Path) -> Result<Vec<Vec<u8>>, StorageError> {
    read_lines(&RealFs::new(), segment)
}

/// Transfers complete lines into the result; the caller moves that result.
pub(super) fn read_lines(vfs: &dyn Vfs, segment: &Path) -> Result<Vec<Vec<u8>>, StorageError> {
    let mut lines = Vec::new();
    scan(vfs, segment, 0, |line, _end| {
        if !line.is_empty() {
            #[cfg(test)]
            let source = line.as_ptr();
            lines.push(std::mem::take(line));
            #[cfg(test)]
            MEASURED.with(|measured| {
                let (peak, copied, copies) = measured.get();
                measured.set((
                    peak,
                    copied,
                    copies.saturating_add(usize::from(
                        lines.last().is_some_and(|result| result.as_ptr() != source),
                    )),
                ));
            });
        }
        Ok(())
    })?;
    Ok(lines)
}

/// Every complete line of the ledger in `dir`, copied out. For callers
/// that keep the lines themselves (fork, replay, fixtures); a fold that
/// wants only what the lines say walks `LedgerIndex::folding` instead.
pub fn read_raw_lines_at(dir: &Path) -> Result<Vec<Vec<u8>>, StorageError> {
    let mut out = Vec::new();
    for segment in ledger_segments_at(dir)? {
        out.extend(read_segment(&segment)?);
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
    static MEASURED: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}

#[cfg(test)]
pub(crate) fn measure(held: usize, copied: usize) {
    MEASURED.with(|measured| {
        let (peak, bytes, copies) = measured.get();
        measured.set((peak.max(held), bytes.saturating_add(copied), copies));
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
    fn streaming_preserves_complete_lines_across_window_boundaries() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("segment");
        let window = usize::try_from(SCAN_WINDOW_BYTES).unwrap();
        for cut in [window.saturating_sub(1), window, window.saturating_add(1)] {
            let mut bytes = vec![b'x'; cut];
            bytes.extend_from_slice(b"\n\n");
            bytes.resize(bytes.len().saturating_add(window.saturating_mul(2)), 0);
            bytes.extend_from_slice(b"after zeros\nunterminated");
            bytes.resize(bytes.len().saturating_add(window), 0);
            std::fs::write(&path, &bytes).unwrap();
            let mut visited = Vec::new();
            let end = scan(&RealFs::new(), &path, 0, |line, end| {
                visited.push((line.to_vec(), end));
                Ok(())
            })
            .unwrap();
            let mut offset = 0u64;
            let expected: Vec<_> = super::super::ledger::complete_lines(&bytes)
                .0
                .into_iter()
                .map(|line| {
                    offset = offset
                        .saturating_add(u64::try_from(line.len()).unwrap())
                        .saturating_add(1);
                    (line.to_vec(), offset)
                })
                .collect();
            assert_eq!(
                (visited, end.total, end.filled),
                (
                    expected,
                    u64::try_from(bytes.len()).unwrap(),
                    u64::try_from(super::super::ledger::records_end(&bytes)).unwrap()
                )
            );
        }
    }

    #[test]
    fn reusing_a_prefix_stays_bounded_and_a_changed_digest_falls_back() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("ledger");
        let proofs = root.path().join("proofs");
        let (mut ledger, _) = JsonlLedger::open(&dir, TimeMs::new(0)).unwrap();
        ledger
            .append_all(
                (0..500)
                    .map(|t| EventDraft {
                        run: RunId::CITY,
                        t: TimeMs::new(t),
                        who: "city".into(),
                        addr: None,
                        kind: EventKind::GateChecked,
                        data: Payload::empty(),
                        ig: false,
                    })
                    .collect(),
            )
            .unwrap();
        crate::chain_audit::prove_chain(&dir, &ledger.proof_records(&proofs)).unwrap();
        let expected = ledger.read_raw_lines().unwrap();
        let segment = ledger_segments_at(&dir).unwrap().remove(0);
        let proved = std::fs::metadata(&segment).unwrap().len();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&segment)
            .unwrap()
            .set_len(2 * 1024 * 1024)
            .unwrap();
        drop(ledger);
        MEASURED.with(|m| m.set((0, 0, 0)));
        let records = crate::verified_prefix::ProofRecords::read_only(&proofs);
        let (reopened, report) = JsonlLedger::open_reusing(&dir, TimeMs::new(1), &records).unwrap();
        let peak = MEASURED.with(std::cell::Cell::get).0;
        assert!(peak < 2 * 1024 * 1024 / 8, "prefix scan retained {peak}");
        assert_eq!(
            (
                report.counted.lines_checked,
                report.counted.segments_by_digest,
                report.counted.bytes_hashed
            ),
            (0, 1, proved)
        );
        assert_eq!(reopened.read_raw_lines().unwrap(), expected);
        drop(reopened);
        let bytes = std::fs::read(&segment).unwrap();
        let changed =
            String::from_utf8(bytes)
                .unwrap()
                .replacen("\"who\":\"city\"", "\"who\":\"town\"", 1);
        std::fs::write(&segment, changed).unwrap();
        let refused = JsonlLedger::open_reusing(&dir, TimeMs::new(2), &records);
        assert!(matches!(
            refused,
            Err(StorageError::Envelope { line: 2, .. })
        ));
    }

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
        let baseline = std::fs::read(&segment).unwrap();
        let mut reference_copies = 0usize;
        let reference: Vec<Vec<u8>> = super::super::ledger::complete_lines(&baseline)
            .0
            .into_iter()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let result = line.to_vec();
                reference_copies =
                    reference_copies.saturating_add(usize::from(result.as_ptr() != line.as_ptr()));
                result
            })
            .collect();
        assert_eq!(reference, expected);
        assert!(reference_copies > 0);
        MEASURED.with(|m| m.set((0, 0, 0)));
        assert_eq!(ledger.read_raw_lines().unwrap(), expected);
        let instance = MEASURED.with(std::cell::Cell::get);
        MEASURED.with(|m| m.set((0, 0, 0)));
        assert_eq!(read_raw_lines_at(&dir).unwrap(), expected);
        let readonly = MEASURED.with(std::cell::Cell::get);
        drop(ledger);
        MEASURED.with(|m| m.set((0, 0, 0)));
        let (reopened, report) = JsonlLedger::open(&dir, TimeMs::new(1)).unwrap();
        assert!(report.recovered.is_none());
        assert_eq!(reopened.position().value(), 1);
        let opening = MEASURED.with(std::cell::Cell::get);
        for (name, (peak, copied, copies)) in [
            ("instance", instance),
            ("readonly", readonly),
            ("open", opening),
        ] {
            eprintln!(
                "{name}: peak={peak} copied_into_scan={copied} result_copies={copies} segment={total}"
            );
            assert!(
                copies < reference_copies,
                "{name}: copied a line instead of moving it"
            );
            assert!(peak < baseline.capacity() / 8, "{name}: retained {peak}");
            assert!(copied < baseline.len() / 8, "{name}: copied {copied}");
        }
    }
}
