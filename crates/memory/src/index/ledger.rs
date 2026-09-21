// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The side index: seq to (segment, byte offset).

use std::ops::Bound;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use kernel::{RunId, Seq};

use crate::error::MemoryError;
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::fold::Folded;
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
///
/// The maps live in [`Folded`]; what this type adds is the filesystem
/// seam they are folded from. The seam sits behind a lock so a caller
/// holding `&LedgerIndex` asks questions without exclusive access, and
/// every question this type answers is a read: nothing here writes.
pub struct LedgerIndex {
    pub(crate) folded: Folded,
    vfs: Mutex<Box<dyn Vfs>>,
}

impl LedgerIndex {
    /// The seam, whatever a thread that held it did before it died: a
    /// disposable side artifact has no invariant a panic could leave
    /// half-kept, so a poisoned lock is taken as it stands — the same
    /// stance `FaultFs` takes on its own state.
    fn seam(&self) -> MutexGuard<'_, Box<dyn Vfs>> {
        self.vfs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Scans the ledger directory into a fresh index.
    ///
    /// # Errors
    /// Propagates a ledger directory that cannot be listed or read.
    pub fn rebuild(dir: &Path) -> Result<LedgerIndex, MemoryError> {
        let vfs: Box<dyn Vfs> = Box::new(RealFs::new());
        let folded = super::fold::rebuild(vfs.as_ref(), dir)?;
        Ok(LedgerIndex {
            folded,
            vfs: Mutex::new(vfs),
        })
    }

    /// An index over nothing.
    ///
    /// For a holder that wants one before the ledger directory exists: the
    /// first `refresh` fills it, and until then every lookup answers
    /// `SeqMissing`, which is the truth.
    #[must_use]
    pub fn empty() -> LedgerIndex {
        LedgerIndex {
            folded: Folded::empty(),
            vfs: Mutex::new(Box::new(RealFs::new())),
        }
    }

    /// Folds whatever has been appended since the last look.
    ///
    /// A resident index is the point: rebuilding one scans every segment
    /// and allocates a `String` for every line in it, which on a fifty
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
    ///
    /// # Errors
    /// Propagates a ledger directory that cannot be listed, stat'ed or
    /// read.
    pub fn refresh(&mut self, dir: &Path) -> Result<Refreshed, MemoryError> {
        let plan = self.plan_refresh(dir)?;
        match plan {
            RefreshPlan::Rebuild => {
                let folded = super::fold::rebuild(self.seam().as_ref(), dir)?;
                self.folded = folded;
                Ok(Refreshed::Rebuilt)
            }
            RefreshPlan::Unchanged => Ok(Refreshed::Unchanged),
            RefreshPlan::Appended { spans } => self.fold_spans(dir, spans),
        }
    }

    /// What this refresh has to do, decided from segment lengths alone.
    fn plan_refresh(&self, dir: &Path) -> Result<RefreshPlan, MemoryError> {
        let vfs = self.seam();
        let names = crate::jsonl::segment_names(vfs.as_ref(), dir)?;
        let mut spans = Vec::new();
        for name in &names {
            let path = dir.join(name);
            let size = vfs
                .size(&path)
                .map_err(crate::error::io_err("stat segment", &path))?;
            match self.folded.scanned.get(name).copied() {
                Some(done) if done == size => continue,
                Some(done) if done < size => spans.push(Span {
                    name: name.clone(),
                    from: done,
                    size,
                }),
                Some(_) => return Ok(RefreshPlan::Rebuild),
                None => spans.push(Span {
                    name: name.clone(),
                    from: 0,
                    size,
                }),
            }
        }
        if self.folded.scanned.keys().any(|held| !names.contains(held)) {
            return Ok(RefreshPlan::Rebuild);
        }
        if spans.is_empty() {
            return Ok(RefreshPlan::Unchanged);
        }
        Ok(RefreshPlan::Appended { spans })
    }

    /// Reads each appended span and folds it. The read is bounded by the
    /// length this refresh stat'ed, so a record appended between the
    /// stat and the read stays for the next refresh rather than being
    /// folded at an offset this pass never confirmed.
    fn fold_spans(&mut self, dir: &Path, spans: Vec<Span>) -> Result<Refreshed, MemoryError> {
        let mut bytes_read: u64 = 0;
        for span in spans {
            let path = dir.join(&span.name);
            let tail = self
                .seam()
                .read_at(&path, span.from, span.size.saturating_sub(span.from))
                .map_err(crate::error::io_err("read segment", &path))?;
            bytes_read = bytes_read.saturating_add(u64::try_from(tail.len()).unwrap_or(u64::MAX));
            let indexed = self.folded.fold_segment(&span.name, span.from, &tail);
            // Only complete lines count as scanned: a torn tail is
            // overwritten by the next append, and remembering it as read
            // would skip the record that replaces it.
            self.folded
                .scanned
                .insert(span.name, span.from.saturating_add(indexed).min(span.size));
        }
        Ok(Refreshed::Appended { bytes_read })
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
        self.folded
            .runs
            .get(&run)
            .into_iter()
            .flat_map(move |seqs| seqs.range((Bound::Unbounded, upper)).rev().copied())
    }

    pub fn tail_seq(&self) -> Option<Seq> {
        self.folded.entries.keys().next_back().copied()
    }

    pub fn len(&self) -> usize {
        self.folded.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.folded.entries.is_empty()
    }
}

/// One segment's appended stretch: where this index stopped, and where
/// the segment now ends.
struct Span {
    name: String,
    from: u64,
    size: u64,
}

/// What a refresh decided before it read anything.
enum RefreshPlan {
    Unchanged,
    Appended { spans: Vec<Span> },
    Rebuild,
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
