// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Index readers: seeking lines without scanning.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use kernel::Seq;

use super::ledger::LedgerIndex;
use crate::error::{MemoryError, io_err};

/// Reads single lines by seq, holding one segment open across reads.
///
/// One handle and one buffer serve the whole stretch, and a walk that
/// goes forward never seeks at all — the position the previous line
/// left is the position the next one wants. What this replaces opened
/// the segment again for every line and then read it one byte at a
/// time. Over a fifty-thousand record ledger on one windows-x86_64
/// NVMe machine (2026-09-02): 734 µs a line then, against 0.89 µs
/// walking forward and 5.82 µs walking backward now.
pub struct LineReader<'index> {
    pub(crate) index: &'index LedgerIndex,
    pub(crate) dir: PathBuf,
    pub(crate) open: Option<OpenSegment>,
}

impl LineReader<'_> {
    /// One line, without its terminator. A seq absent from the index is
    /// a caller error, not a corrupt ledger.
    pub fn line_at(&mut self, seq: Seq) -> Result<Vec<u8>, MemoryError> {
        let Some((name, offset)) = self.index.entries.get(&seq) else {
            return Err(MemoryError::SeqMissing { seq: seq.value() });
        };
        let offset = *offset;
        let segment = match self.open.take() {
            Some(open) if open.name == *name => open,
            _ => OpenSegment::open(&self.dir, name)?,
        };
        self.open.insert(segment).line_at(offset)
    }
}

/// The segment a `LineReader` currently holds open, and where reading
/// it would resume.
pub(crate) struct OpenSegment {
    name: String,
    path: PathBuf,
    file: BufReader<File>,
    /// The offset the next read starts at, while that offset is known.
    /// `None` means the position must be sought before it is used —
    /// a remembered position that might be wrong would hand the caller
    /// another line's bytes under the seq it asked for.
    resume: Option<u64>,
}

impl OpenSegment {
    fn open(dir: &Path, name: &str) -> Result<OpenSegment, MemoryError> {
        let path = dir.join(name);
        let file = File::open(&path).map_err(io_err("open segment", &path))?;
        Ok(OpenSegment {
            name: name.to_owned(),
            path,
            file: BufReader::new(file),
            resume: Some(0),
        })
    }

    fn line_at(&mut self, offset: u64) -> Result<Vec<u8>, MemoryError> {
        if self.resume != Some(offset) {
            self.resume = None;
            self.file
                .seek(SeekFrom::Start(offset))
                .map_err(io_err("seek segment", &self.path))?;
        }
        self.resume = None;
        let mut line = Vec::new();
        let read = self
            .file
            .read_until(b'\n', &mut line)
            .map_err(io_err("read segment", &self.path))?;
        self.resume = u64::try_from(read)
            .ok()
            .and_then(|len| offset.checked_add(len));
        if line.last().copied() == Some(b'\n') {
            line.pop();
        }
        Ok(line)
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
    use kernel::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, TimeMs};
    /// Writes `count` records into segments that roll every
    /// `per_segment` lines, and returns each line in seq order.
    fn write_ledger_rolling(dir: &Path, count: u64, per_segment: u64) -> Vec<Vec<u8>> {
        let run = RunId::from_bytes([7u8; 16]);
        let mut lines = Vec::new();
        let mut blob = Vec::new();
        let mut first_seq = 0u64;
        let mut prev = B3Hash::digest(b"");
        for i in 0..count {
            let draft = EventDraft {
                run,
                t: TimeMs::new(i),
                who: "tester".to_owned(),
                addr: None,
                kind: EventKind::RunStarted,
                data: Payload::new(serde_json::Map::new()).unwrap(),
                ig: false,
            };
            let record = EventRecord::from_draft(draft, Seq::new(i), prev);
            let line = record.canonical_line().unwrap();
            prev = B3Hash::digest(&line);
            blob.extend_from_slice(&line);
            blob.push(b'\n');
            lines.push(line);
            let next = i.saturating_add(1);
            if next.checked_rem(per_segment) == Some(0) && next < count {
                std::fs::write(dir.join(format!("ledger-{first_seq:020}.jsonl")), &blob).unwrap();
                blob.clear();
                first_seq = next;
            }
        }
        std::fs::write(dir.join(format!("ledger-{first_seq:020}.jsonl")), &blob).unwrap();
        lines
    }

    fn write_interleaved(dir: &Path, runs: &[RunId], count: u64) -> Vec<(RunId, Vec<Seq>)> {
        let mut blob = Vec::new();
        let mut prev = B3Hash::digest(b"");
        let mut owned: Vec<(RunId, Vec<Seq>)> = runs.iter().map(|run| (*run, Vec::new())).collect();
        for i in 0..count {
            let slot = usize::try_from(i)
                .unwrap()
                .checked_rem(runs.len())
                .expect("at least one run to cycle through");
            let run = runs[slot];
            let draft = EventDraft {
                run,
                t: TimeMs::new(i),
                who: "tester".to_owned(),
                addr: None,
                kind: EventKind::RunStarted,
                data: Payload::new(serde_json::Map::new()).unwrap(),
                ig: false,
            };
            let record = EventRecord::from_draft(draft, Seq::new(i), prev);
            let line = record.canonical_line().unwrap();
            prev = B3Hash::digest(&line);
            blob.extend_from_slice(&line);
            blob.push(b'\n');
            owned[slot].1.push(Seq::new(i));
        }
        std::fs::write(dir.join("ledger-00000000000000000000.jsonl"), &blob).unwrap();
        owned
    }

    /// The whole point of the run index: one session's lines are found
    /// without reading anybody else's. The assertion is on the seqs the
    /// index hands back, because that set *is* what the reader will open
    /// the segment for - a walk that returned the other runs' seqs and
    /// left the filtering to the caller would have read them all.
    #[test]
    fn one_reader_answers_out_of_order_seeks_across_segments() {
        let tmp = tempfile::tempdir().unwrap();
        let lines = write_ledger_rolling(tmp.path(), 9, 4);
        let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
        assert_eq!(index.len(), 9, "three segments, nine lines");
        let mut reader = index.reader(tmp.path());
        // Backwards, repeated, and ping-ponging between segments. A
        // reader that holds a handle open must hold its position honest
        // too: every one of these asks for a line the buffer is not
        // already sitting on.
        for value in [8u64, 0, 8, 3, 4, 3, 7, 1, 1, 5] {
            let got = reader.line_at(Seq::new(value)).unwrap();
            let expected = &lines[usize::try_from(value).unwrap()];
            assert_eq!(&got, expected, "seq {value} reads back verbatim");
        }
    }

    #[test]
    fn a_run_s_own_lines_are_found_without_walking_anybody_else_s() {
        let tmp = tempfile::tempdir().unwrap();
        let mine = RunId::from_bytes([1u8; 16]);
        let yours = RunId::from_bytes([2u8; 16]);
        let city = RunId::CITY;
        let owned = write_interleaved(tmp.path(), &[mine, yours, city], 30);
        let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();

        let all: Vec<Seq> = index.run_seqs_before(mine, None).collect();
        let mut expected = owned[0].1.clone();
        expected.reverse();
        assert_eq!(all, expected, "newest first, and only this run's lines");

        // Bounded by the answer rather than by the ledger: taking two
        // reads two, however many lines the other runs wrote between.
        let newest_two: Vec<Seq> = index.run_seqs_before(mine, None).take(2).collect();
        assert_eq!(newest_two, [Seq::new(27), Seq::new(24)]);

        // `before` is exclusive, which is what the wire's `earlier`
        // cursor already means; paging with it never repeats a record.
        let older: Vec<Seq> = index
            .run_seqs_before(mine, Some(Seq::new(24)))
            .take(2)
            .collect();
        assert_eq!(older, [Seq::new(21), Seq::new(18)]);

        // A session this ledger never held has no lines, which is a
        // different answer from an empty ledger and needs no special case.
        let stranger: Vec<Seq> = index
            .run_seqs_before(RunId::from_bytes([9u8; 16]), None)
            .collect();
        assert!(stranger.is_empty());

        // The city's own records are a run like any other here.
        assert_eq!(index.run_seqs_before(city, None).count(), 10);
    }

    #[test]
    fn the_run_map_survives_a_refresh_and_a_cache_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let mine = RunId::from_bytes([1u8; 16]);
        let yours = RunId::from_bytes([2u8; 16]);
        write_interleaved(tmp.path(), &[mine, yours], 4);
        let mut index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
        assert_eq!(index.run_seqs_before(mine, None).count(), 2);

        // Appended: the refresh folds the new lines into both maps.
        write_interleaved(tmp.path(), &[mine, yours], 10);
        index.refresh(tmp.path()).unwrap();
        assert_eq!(index.run_seqs_before(mine, None).count(), 5);

        // Persisted and read back: a cache that carried offsets but not
        // runs would answer this with nothing.
        index.persist(tmp.path()).unwrap();
        let loaded = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
        assert_eq!(loaded.len(), 10, "the cache was believed, not rebuilt");
        assert_eq!(loaded.run_seqs_before(mine, None).count(), 5);
    }
}
