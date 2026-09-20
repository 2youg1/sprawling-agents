// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The side index: seq to (segment, byte offset).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Bound;
use std::path::Path;

use kernel::{RunId, Seq};

use crate::error::{MemoryError, io_err};

use super::reader::LineReader;

/// What one [`LedgerIndex::refresh`] did, and what it lifted off the
/// disk doing it.
///
/// The byte count is the difference between reading the appended tail
/// and reading the segment that carries it, so it is the reading the
/// `index_refresh_read` budget records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refreshed {
    /// Every segment was exactly the length this index had already
    /// folded, so nothing was opened.
    Unchanged,
    /// Only the bytes appended since the last look were read.
    Appended { bytes_read: u64 },
    /// A segment shrank or vanished, so every offset came from a full
    /// scan of the directory.
    Rebuilt,
}

/// seq → (segment file name, byte offset of the line start).
pub struct LedgerIndex {
    pub(crate) entries: BTreeMap<Seq, (String, u64)>,
    /// run → the sequences it wrote.
    ///
    /// The same map read the other way round, and the reason one
    /// session's history costs its own length rather than the ledger's:
    /// without it, answering "the newest twenty lines of this session"
    /// meant reading every line back to wherever the answer ran out and
    /// discarding what belonged to somebody else.
    pub(crate) runs: BTreeMap<RunId, BTreeSet<Seq>>,
    /// Bytes of each segment already folded into `entries`.
    ///
    /// The one home of "how far this index has read", and it always
    /// names the end of a complete line: `fold_segment` counts only
    /// lines that carried their terminator, so the next refresh starts
    /// where a record starts and can never lift half of one.
    ///
    /// It is resident state, never read back off the disk. A cache is
    /// believed only when the directory's byte count and digest match
    /// its stamp, and at that moment this map is the segment sizes
    /// themselves; a cache whose offsets were damaged fails the stamp
    /// and the whole index is rebuilt. A segment that shrank has the
    /// same answer, because tail recovery truncates and every offset
    /// held for that segment then describes bytes that are gone.
    pub(crate) scanned: BTreeMap<String, u64>,
}

impl LedgerIndex {
    /// Reads `size - from` bytes of `path`, starting at `from`, and
    /// leaves everything before `from` on the disk.
    ///
    /// `take` bounds the read by the length this refresh stat'ed, so a
    /// record appended between the stat and the read stays for the next
    /// refresh rather than being folded at an offset this pass never
    /// confirmed.
    ///
    /// `None` asks the caller to rebuild: it means the span does not fit
    /// this machine's pointer width, which no ledger this crate writes
    /// can reach, and a rebuild is the answer that cannot be wrong.
    fn read_span(path: &Path, from: u64, size: u64) -> Result<Option<Vec<u8>>, MemoryError> {
        let span = size.saturating_sub(from);
        let Ok(capacity) = usize::try_from(span) else {
            return Ok(None);
        };
        let mut file = File::open(path).map_err(io_err("open segment", path))?;
        file.seek(SeekFrom::Start(from))
            .map_err(io_err("seek segment", path))?;
        let mut tail = Vec::with_capacity(capacity);
        file.take(span)
            .read_to_end(&mut tail)
            .map_err(io_err("read segment", path))?;
        Ok(Some(tail))
    }
    /// Loads the cache when it is checksum-fresh, otherwise scans the
    /// directory. Both paths yield the same map; only the cost differs.
    pub fn load_or_rebuild(dir: &Path) -> Result<LedgerIndex, MemoryError> {
        let stamp = super::cache::directory_stamp(dir)?;
        if let Some(index) = super::cache::load_cache(dir, &stamp) {
            return Ok(index);
        }
        super::cache::rebuild(dir)
    }

    /// An index over nothing.
    ///
    /// For a holder that wants one before the ledger directory exists: the
    /// first `refresh` fills it, and until then every lookup answers
    /// `SeqMissing`, which is the truth.
    #[must_use]
    pub fn empty() -> LedgerIndex {
        LedgerIndex {
            entries: BTreeMap::new(),
            runs: BTreeMap::new(),
            scanned: BTreeMap::new(),
        }
    }

    /// Folds whatever has been appended since the last look.
    ///
    /// A resident index is the point: rebuilding one costs a read of the
    /// whole cache and a `String` for every line in it, which on a fifty
    /// thousand record ledger was 14.4 ms charged to every single
    /// history query. Refreshing costs one directory listing plus a
    /// `stat` per segment, and reads only the bytes that are new.
    ///
    /// The incremental face is here rather than an "index, a record just
    /// landed" call on the writer's side, because a caller holds an
    /// `EventRecord` and segments are this crate's own business. Handing
    /// an offset to an observer would leak the layout to everyone.
    ///
    /// A segment that shrank or vanished forces a full rebuild: tail
    /// recovery truncates, and after it the offsets held for that
    /// segment describe bytes that are no longer there.
    ///
    /// The answer reports how many bytes this refresh read, which is
    /// what separates seeking to the tail from lifting the segment that
    /// holds it.
    pub fn refresh(&mut self, dir: &Path) -> Result<Refreshed, MemoryError> {
        let names = super::cache::segment_names(dir)?;
        let mut fresh = Vec::new();
        for name in &names {
            let path = dir.join(name);
            let size = std::fs::metadata(&path)
                .map_err(io_err("stat segment", &path))?
                .len();
            match self.scanned.get(name).copied() {
                Some(done) if done == size => continue,
                Some(done) if done < size => fresh.push((name.clone(), done, size)),
                Some(_) => return self.replace_with(super::cache::rebuild(dir)?),
                None => fresh.push((name.clone(), 0, size)),
            }
        }
        if self.scanned.keys().any(|held| !names.contains(held)) {
            return self.replace_with(super::cache::rebuild(dir)?);
        }
        if fresh.is_empty() {
            return Ok(Refreshed::Unchanged);
        }
        let mut bytes_read: u64 = 0;
        for (name, from, size) in fresh {
            let path = dir.join(&name);
            let Some(tail) = LedgerIndex::read_span(&path, from, size)? else {
                return self.replace_with(super::cache::rebuild(dir)?);
            };
            bytes_read = bytes_read.saturating_add(u64::try_from(tail.len()).unwrap_or(u64::MAX));
            let indexed = self.fold_segment(&name, from, &tail);
            // Only complete lines count as scanned: a torn tail is
            // overwritten by the next append, and remembering it as read
            // would skip the record that replaces it.
            self.scanned
                .insert(name, from.saturating_add(indexed).min(size));
        }
        Ok(Refreshed::Appended { bytes_read })
    }

    /// Indexes the complete lines of `tail`, which begins at `from` in
    /// the segment. Returns how many bytes those complete lines took.
    pub(crate) fn fold_segment(&mut self, name: &str, from: u64, tail: &[u8]) -> u64 {
        let mut offset = from;
        let mut complete_bytes = 0u64;
        for line in tail.split_inclusive(|b| *b == b'\n') {
            if line.last().copied() != Some(b'\n') {
                break;
            }
            let body = line.get(..line.len().saturating_sub(1)).unwrap_or(line);
            if !body.is_empty() {
                self.insert_line(name, offset, body);
            }
            let len = u64::try_from(line.len()).unwrap_or(0);
            offset = offset.saturating_add(len);
            complete_bytes = complete_bytes.saturating_add(len);
        }
        complete_bytes
    }

    /// What indexing one line means, in one place: where it sits, and
    /// whose it is. A line that does not parse is skipped rather than
    /// reported — the caller is looking at a ledger that may be damaged,
    /// and an index is how such a ledger gets repaired.
    pub(crate) fn insert_line(&mut self, name: &str, offset: u64, body: &[u8]) {
        let Some(located) = super::cache::locate(body) else {
            return;
        };
        self.entries.insert(located.seq, (name.to_owned(), offset));
        if let Some(run) = located.run {
            self.runs.entry(run).or_default().insert(located.seq);
        }
    }

    fn replace_with(&mut self, built: LedgerIndex) -> Result<Refreshed, MemoryError> {
        *self = built;
        Ok(Refreshed::Rebuilt)
    }

    /// A cursor for reading lines out of this index.
    ///
    /// The unit of work is a stretch of sequences rather than one of
    /// them, so the handle belongs to the stretch: a caller asks for a
    /// reader once and then for lines as often as it likes.
    pub fn reader(&self, dir: &Path) -> LineReader<'_> {
        LineReader {
            index: self,
            dir: dir.to_path_buf(),
            open: None,
        }
    }

    /// The sequences one run wrote, newest first, below `before`.
    ///
    /// Newest first because what a reader opens a session for is its end;
    /// a caller that wants them oldest first takes what it needs and
    /// reverses a list it already holds, which also lets it read the
    /// segment forwards.
    ///
    /// `before` is exclusive, the same word and the same meaning the wire
    /// gives `HistoryAnswer::earlier`, so paging by handing one answer's
    /// cursor to the next question can neither repeat nor skip a record.
    ///
    /// A run this index never saw yields nothing, which is the truth and
    /// needs no separate answer.
    pub fn run_seqs_before(&self, run: RunId, before: Option<Seq>) -> impl Iterator<Item = Seq> {
        let upper = match before {
            Some(seq) => Bound::Excluded(seq),
            None => Bound::Unbounded,
        };
        self.runs
            .get(&run)
            .into_iter()
            .flat_map(move |seqs| seqs.range((Bound::Unbounded, upper)).rev().copied())
    }

    /// Writes the cache. Failure is not fatal — the index rebuilds next
    /// time, and a disposable artifact must never block the main path.
    pub fn persist(&self, dir: &Path) -> Result<(), MemoryError> {
        let stamp = super::cache::directory_stamp(dir)?;
        // The run map inverted for the length of this write. Held here
        // rather than resident because a cache row is the only reader of
        // "which run owns this seq", and a second resident copy would be
        // a second thing to keep in step.
        let mut owner: BTreeMap<Seq, RunId> = BTreeMap::new();
        for (run, seqs) in &self.runs {
            for seq in seqs {
                owner.insert(*seq, *run);
            }
        }
        let mut out = format!(
            "{} {} {}\n",
            super::cache::CACHE_MAGIC,
            stamp.bytes,
            stamp.digest
        );
        for (seq, (segment, offset)) in &self.entries {
            let run = match owner.get(seq) {
                Some(run) => run.to_string(),
                None => super::cache::NO_RUN.to_owned(),
            };
            out.push_str(&format!("{} {segment} {offset} {run}\n", seq.value()));
        }
        let path = dir.join(super::cache::CACHE_NAME);
        std::fs::write(&path, out.as_bytes()).map_err(io_err("write index cache", &path))
    }

    pub fn tail_seq(&self) -> Option<Seq> {
        self.entries.keys().next_back().copied()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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
mod tests;
