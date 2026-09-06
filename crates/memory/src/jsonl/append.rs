// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ledger appends: waves, reads, and the kernel Ledger face.

use std::path::{Path, PathBuf};

use kernel::{
    AxCode, AxError, EventDraft, EventKind, EventRecord, EventRef, Payload, RunId, Seq, TimeMs,
    chain_hash,
};

use crate::error::{MemoryError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::ledger::{JsonlLedger, WriteObserver, complete_lines, is_segment, segment_file_name};

impl JsonlLedger {
    pub(crate) fn append_log_truncated(
        &mut self,
        now: TimeMs,
        dropped: u64,
    ) -> Result<(), MemoryError> {
        let mut map = serde_json::Map::new();
        map.insert("dropped_bytes".to_owned(), serde_json::Value::from(dropped));
        let data = Payload::new(map).map_err(|source| MemoryError::Draft { source })?;
        let draft = EventDraft {
            run: RunId::CITY,
            t: now,
            who: "system".to_owned(),
            addr: None,
            kind: EventKind::LogTruncated,
            data,
            ig: false,
        };
        self.append_all(vec![draft]).map(|_| ())
    }

    /// Group commit: one durability barrier for the whole wave
    /// (memory-SPEC 3-1: the batch is what the wave delivered).
    pub fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, MemoryError> {
        if drafts.is_empty() {
            return Ok(Vec::new());
        }
        let mut seq = self.next_seq;
        let mut prev = self.prev;
        let mut cur_path = self.seg_path.clone();
        let mut cur_len = self.seg_len;
        let mut writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();
        let mut created: Vec<PathBuf> = Vec::new();
        let mut records = Vec::with_capacity(drafts.len());

        if cur_len == 0 && !self.vfs.exists(&cur_path) {
            created.push(cur_path.clone());
        }

        for draft in drafts {
            let record = EventRecord::from_draft(draft, seq, prev);
            let line = record
                .canonical_line()
                .map_err(|source| MemoryError::Draft { source })?;
            let line_len = u64::try_from(line.len())
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            if cur_len > 0 && cur_len.saturating_add(line_len) > self.roll_bytes {
                cur_path = self.dir.join(segment_file_name(seq));
                cur_len = 0;
                created.push(cur_path.clone());
            }
            prev = chain_hash(&line);
            seq = seq.next().map_err(|source| MemoryError::Draft { source })?;
            records.push(record);
            let mut terminated = line;
            terminated.push(b'\n');
            match writes.last_mut() {
                Some((path, buffer)) if *path == cur_path => buffer.extend_from_slice(&terminated),
                _ => writes.push((cur_path.clone(), terminated)),
            }
            cur_len = cur_len.saturating_add(line_len);
        }

        for (path, bytes) in &writes {
            self.vfs
                .append(path, bytes)
                .map_err(io_err("append event line", path))?;
        }
        for (path, _) in &writes {
            self.vfs
                .sync_data(path)
                .map_err(io_err("sync segment", path))?;
        }
        if !created.is_empty() {
            self.vfs
                .sync_dir(&self.dir.clone())
                .map_err(io_err("sync ledger dir", &self.dir.clone()))?;
        }

        self.seg_path = cur_path;
        self.seg_len = cur_len;
        self.next_seq = seq;
        self.prev = prev;

        // Only now, with the bytes synced, does anyone else hear about
        // them: an observer that saw an event the disk never got would be
        // telling the interface something the history does not contain.
        if let Some(observer) = self.observer.as_mut() {
            for record in &records {
                observer(record);
            }
        }
        Ok(records.iter().map(EventRecord::to_ref).collect())
    }

    /// The position a record written now would take.
    ///
    /// A position, never content: this is what anchors a diagnostic log
    /// line to the only history (`docs/logging.md` section 4), and a
    /// reader accessor that returned records would invite decision logic
    /// to consult the ledger it is in the middle of writing.
    #[must_use]
    pub fn position(&self) -> Seq {
        self.next_seq
    }

    /// Installs the write-path observer, replacing any earlier one. The
    /// sink runs on the appending thread after durability, so it must not
    /// block; a bounded send that drops on lag is the shape this expects.
    pub fn observe(&mut self, sink: WriteObserver) {
        self.observer = Some(sink);
    }

    /// The read face for replay, fixtures and inspection: every canonical
    /// line (no terminators), segments flattened in order.
    pub fn read_raw_lines(&self) -> Result<Vec<Vec<u8>>, MemoryError> {
        let mut out = Vec::new();
        let segments: Vec<PathBuf> = self
            .vfs
            .list(&self.dir)
            .map_err(io_err("list ledger dir", &self.dir))?
            .into_iter()
            .filter(|p| is_segment(p))
            .collect();
        for seg in segments {
            let bytes = self.vfs.read(&seg).map_err(io_err("read segment", &seg))?;
            let (lines, _) = complete_lines(&bytes);
            for line in lines {
                if !line.is_empty() {
                    out.push(line.to_vec());
                }
            }
        }
        Ok(out)
    }

    #[cfg(test)]
    pub(crate) fn set_roll_bytes_for_test(&mut self, roll_bytes: u64) {
        self.roll_bytes = roll_bytes;
    }
}

/// Read-only face for replay and fixtures: never opens the ledger, never
/// repairs, never writes — replay must not mutate what it verifies
/// (runtime-SPEC 8-1). Complete lines only; a torn tail byte-run is not a
/// line and is left for `open` to judge.
pub fn read_raw_lines_at(dir: &Path) -> Result<Vec<Vec<u8>>, MemoryError> {
    let vfs = RealFs::new();
    let mut out = Vec::new();
    for seg in ledger_segments_at(dir)? {
        let bytes = vfs.read(&seg).map_err(io_err("read segment", &seg))?;
        let (lines, _) = complete_lines(&bytes);
        for line in lines {
            if !line.is_empty() {
                out.push(line.to_vec());
            }
        }
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
pub fn ledger_segments_at(dir: &Path) -> Result<Vec<PathBuf>, MemoryError> {
    let vfs = RealFs::new();
    Ok(vfs
        .list(dir)
        .map_err(io_err("list ledger dir", dir))?
        .into_iter()
        .filter(|p| is_segment(p))
        .collect())
}

impl kernel::Ledger for JsonlLedger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let refs = self.append_all(vec![draft]).map_err(MemoryError::into_ax)?;
        refs.into_iter()
            .next()
            .ok_or_else(|| AxError::failure(AxCode::InvalidArgs, "append event", "empty wave echo"))
    }
}

#[cfg(feature = "conformance")]
impl kernel::conformance::LedgerInspect for JsonlLedger {
    fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError> {
        self.read_raw_lines().map_err(MemoryError::into_ax)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::GENESIS_PREV;
    use std::fs;
    fn draft(kind: EventKind, t: u64) -> EventDraft {
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(t),
            who: "city".to_string(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        }
    }
    fn verify_chain(lines: &[Vec<u8>]) {
        let mut prev = GENESIS_PREV;
        for (i, line) in lines.iter().enumerate() {
            let record = EventRecord::parse_line(line).unwrap();
            assert_eq!(record.prev(), prev, "prev broken at line {i}");
            assert_eq!(record.seq(), Seq::new(u64::try_from(i).unwrap()));
            prev = chain_hash(line);
        }
    }

    #[cfg(feature = "conformance")]
    #[test]
    fn passes_the_kernel_conformance_suite() {
        use kernel::conformance::assert_ledger_conformance;
        let keep: std::cell::RefCell<Vec<tempfile::TempDir>> = std::cell::RefCell::new(Vec::new());
        assert_ledger_conformance(|| {
            let dir = tempfile::tempdir().unwrap();
            let (ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
            keep.borrow_mut().push(dir);
            ledger
        });
    }

    #[test]
    fn rolls_segments_without_breaking_seq_or_chain() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.set_roll_bytes_for_test(1);
        for t in 0..4 {
            ledger
                .append_all(vec![draft(EventKind::GateChecked, t)])
                .unwrap();
        }
        let segments = fs::read_dir(dir.path()).unwrap().count();
        assert!(segments >= 4, "tiny roll budget must produce many segments");
        let lines = ledger.read_raw_lines().unwrap();
        assert_eq!(lines.len(), 4);
        verify_chain(&lines);

        drop(ledger);
        let (mut reopened, report) = JsonlLedger::open(dir.path(), TimeMs::new(9)).unwrap();
        assert!(report.recovered.is_none());
        reopened
            .append_all(vec![draft(EventKind::RunFrozen, 9)])
            .unwrap();
        verify_chain(&reopened.read_raw_lines().unwrap());
    }

    fn only_segment(dir: &Path) -> std::path::PathBuf {
        let mut files: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        assert_eq!(files.len(), 1);
        files.remove(0)
    }

    proptest::proptest! {
        #[test]
        fn any_tail_damage_recovers_to_a_valid_chain(
            n in 1usize..5,
            cut_back in 1usize..40,
            garbage in proptest::collection::vec(1u8..=255, 0..24),
        ) {
            let dir = tempfile::tempdir().unwrap();
            let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
            let drafts: Vec<_> = (0..n)
                .map(|i| draft(EventKind::GateChecked, u64::try_from(i).unwrap()))
                .collect();
            ledger.append_all(drafts).unwrap();
            let baseline = ledger.read_raw_lines().unwrap();
            let seg = only_segment(dir.path());
            drop(ledger);

            let mut bytes = fs::read(&seg).unwrap();
            let keep = bytes.len().saturating_sub(cut_back);
            bytes.truncate(keep);
            bytes.extend_from_slice(&garbage);
            fs::write(&seg, &bytes).unwrap();

            let (reopened, _) = JsonlLedger::open(dir.path(), TimeMs::new(999)).unwrap();
            let lines = reopened.read_raw_lines().unwrap();
            verify_chain(&lines);
            // Every surviving pre-damage line is a byte-exact prefix entry.
            let survivors = lines
                .iter()
                .filter(|l| {
                    EventRecord::parse_line(l).unwrap().kind() != EventKind::LogTruncated
                })
                .count();
            proptest::prop_assert!(survivors <= baseline.len());
            for (mine, original) in lines.iter().take(survivors).zip(baseline.iter()) {
                proptest::prop_assert_eq!(mine, original);
            }
        }
    }
}
