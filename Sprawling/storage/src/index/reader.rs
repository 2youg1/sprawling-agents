// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use kernel::Seq;

use super::ledger::LedgerIndex;
use crate::error::{StorageError, io_err};

pub struct LineReader<'index> {
    pub(crate) index: &'index LedgerIndex,
    pub(crate) dir: PathBuf,
    pub(crate) open: Option<OpenSegment>,
}

impl LineReader<'_> {
    pub fn line_at(&mut self, seq: Seq) -> Result<Vec<u8>, StorageError> {
        let Some((name, offset)) = self.index.folded.loc_of(seq) else {
            return Err(StorageError::SeqMissing { seq: seq.value() });
        };
        let segment = match self.open.take() {
            Some(open) if open.name == *name => open,
            _ => OpenSegment::open(&self.dir, name)?,
        };
        self.open.insert(segment).line_at(offset)
    }
}

pub(crate) struct OpenSegment {
    name: String,
    path: PathBuf,
    file: BufReader<File>,

    resume: Option<u64>,
}

impl OpenSegment {
    fn open(dir: &Path, name: &str) -> Result<OpenSegment, StorageError> {
        let path = dir.join(name);
        let file = File::open(&path).map_err(io_err("open segment", &path))?;
        Ok(OpenSegment {
            name: name.to_owned(),
            path,
            file: BufReader::new(file),
            resume: Some(0),
        })
    }

    fn line_at(&mut self, offset: u64) -> Result<Vec<u8>, StorageError> {
        if self.resume != Some(offset) {
            let distance = self.resume.and_then(|at| {
                i64::try_from(offset)
                    .ok()?
                    .checked_sub(i64::try_from(at).ok()?)
            });
            self.resume = None;
            let moved = match distance {
                Some(distance) => self.file.seek_relative(distance),
                None => self.file.seek(SeekFrom::Start(offset)).map(|_start| ()),
            };
            moved.map_err(io_err("seek segment", &self.path))?;
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

    #[test]
    fn one_reader_answers_out_of_order_seeks_across_segments() {
        let tmp = tempfile::tempdir().unwrap();
        let lines = write_ledger_rolling(tmp.path(), 9, 4);
        let index = LedgerIndex::rebuild(tmp.path()).unwrap();
        assert_eq!(index.len(), 9, "three segments, nine lines");
        let mut reader = index.reader(tmp.path());

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
        let index = LedgerIndex::rebuild(tmp.path()).unwrap();

        let all: Vec<Seq> = index.run_seqs_before(mine, None).collect();
        let mut expected = owned[0].1.clone();
        expected.reverse();
        assert_eq!(all, expected, "newest first, and only this run's lines");

        let newest_two: Vec<Seq> = index.run_seqs_before(mine, None).take(2).collect();
        assert_eq!(newest_two, [Seq::new(27), Seq::new(24)]);

        let older: Vec<Seq> = index
            .run_seqs_before(mine, Some(Seq::new(24)))
            .take(2)
            .collect();
        assert_eq!(older, [Seq::new(21), Seq::new(18)]);

        let stranger: Vec<Seq> = index
            .run_seqs_before(RunId::from_bytes([9u8; 16]), None)
            .collect();
        assert!(stranger.is_empty());

        assert_eq!(index.run_seqs_before(city, None).count(), 10);
    }

    #[test]
    fn the_run_map_survives_a_refresh() {
        let tmp = tempfile::tempdir().unwrap();
        let mine = RunId::from_bytes([1u8; 16]);
        let yours = RunId::from_bytes([2u8; 16]);
        write_interleaved(tmp.path(), &[mine, yours], 4);
        let mut index = LedgerIndex::rebuild(tmp.path()).unwrap();
        assert_eq!(index.run_seqs_before(mine, None).count(), 2);

        write_interleaved(tmp.path(), &[mine, yours], 10);
        index.refresh(tmp.path()).unwrap();
        assert_eq!(index.run_seqs_before(mine, None).count(), 5);

        let reopened = LedgerIndex::rebuild(tmp.path()).unwrap();
        assert_eq!(reopened.len(), 10);
        assert_eq!(reopened.run_seqs_before(mine, None).count(), 5);
    }
}
