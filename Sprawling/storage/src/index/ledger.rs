// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The side index: seq to (segment, byte offset).

use std::io;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use kernel::{AxError, RunId, Seq};

use crate::error::{StorageError, io_err};
use crate::jsonl::{FIRST_WINDOW_BYTES, records_end, segment_names};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::fold::{Folded, Located};
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
    /// A segment was longer than what this index had folded, so the
    /// bytes past that point were read: up to the segment's end, or up to
    /// the window that reached a preallocated segment's zero tail
    /// (storage D32), which a refresh reads even when no record arrived.
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
    pub fn rebuild(dir: &Path) -> Result<LedgerIndex, StorageError> {
        let vfs: Box<dyn Vfs> = Box::new(RealFs::new());
        let folded = scan(vfs.as_ref(), dir)?;
        Ok(LedgerIndex {
            folded,
            vfs: Mutex::new(vfs),
        })
    }

    /// Reads the ledger once for a fold and for the index together: each
    /// complete line is lent to `each` in ledger order, then indexed at
    /// its offset. `each` returns the line's place when it has read it
    /// already, so the index does not parse the line again, or `None` for
    /// the index to locate it by the rule `rebuild` uses. One segment's
    /// bytes are resident at a time, and the index covers exactly the
    /// lines `each` saw: a line appended during the fold waits for
    /// `refresh`.
    ///
    /// # Errors
    /// The first error `each` returns, or a ledger directory that cannot
    /// be listed or read. The lines before it were already lent, so the
    /// caller discards what it folded.
    pub fn folding(
        dir: &Path,
        each: impl FnMut(&[u8]) -> Result<Option<Located>, AxError>,
    ) -> Result<LedgerIndex, AxError> {
        let vfs: Box<dyn Vfs> = Box::new(RealFs::new());
        let folded = walk(vfs.as_ref(), dir, each, StorageError::into_ax)?;
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
    pub fn refresh(&mut self, dir: &Path) -> Result<Refreshed, StorageError> {
        let plan = self.plan_refresh(dir)?;
        match plan {
            RefreshPlan::Rebuild => {
                let folded = scan(self.seam().as_ref(), dir)?;
                self.folded = folded;
                Ok(Refreshed::Rebuilt)
            }
            RefreshPlan::Unchanged => Ok(Refreshed::Unchanged),
            RefreshPlan::Appended { spans } => self.fold_spans(dir, spans),
        }
    }

    /// What this refresh has to do, decided from segment lengths alone.
    fn plan_refresh(&self, dir: &Path) -> Result<RefreshPlan, StorageError> {
        let vfs = self.seam();
        let names = crate::jsonl::segment_names(vfs.as_ref(), dir)?;
        let mut spans = Vec::new();
        // Folded segments this listing still holds. Names in one listing
        // are distinct, so fewer hits than folded segments means one of
        // them vanished: one ordered lookup per name, not names squared.
        let mut held_here: usize = 0;
        for name in &names {
            let path = dir.join(name);
            let size = vfs
                .size(&path)
                .map_err(crate::error::io_err("stat segment", &path))?;
            let held = self.folded.scanned.get(name).copied();
            if held.is_some() {
                held_here = held_here.saturating_add(1);
            }
            match held {
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
        if held_here < self.folded.scanned.len() {
            return Ok(RefreshPlan::Rebuild);
        }
        if spans.is_empty() {
            return Ok(RefreshPlan::Unchanged);
        }
        Ok(RefreshPlan::Appended { spans })
    }

    /// Reads the records of each appended span and folds them.
    fn fold_spans(&mut self, dir: &Path, spans: Vec<Span>) -> Result<Refreshed, StorageError> {
        let mut bytes_read: u64 = 0;
        for span in spans {
            let path = dir.join(&span.name);
            let (tail, read) = read_records(self.seam().as_ref(), &path, span.from, span.size)
                .map_err(io_err("read segment", &path))?;
            bytes_read = bytes_read.saturating_add(read);
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
    ///
    /// The walk is lazy: taking `n` values costs `n` steps, not the
    /// length of the run.
    pub fn run_seqs_before(&self, run: RunId, before: Option<Seq>) -> impl Iterator<Item = Seq> {
        self.folded.run_seqs_before(run, before)
    }

    /// Every seq the index holds, ascending: the walk a reader takes
    /// to read the whole ledger forwards, which is its fast direction.
    pub fn seqs(&self) -> impl DoubleEndedIterator<Item = Seq> + '_ {
        self.folded.seqs()
    }

    /// Every seq the index holds at or after `from`, ascending: the walk
    /// a reader takes to read a stretch forwards from where it starts.
    ///
    /// The walk starts at `from` rather than skipping the seqs before it,
    /// so reading a stretch costs the stretch, however long the ledger
    /// before it is (`crates/storage/Spec.lean` §8-38). `from` need not be
    /// a seq anybody wrote; nothing at or after it yields nothing.
    pub fn seqs_from(&self, from: Seq) -> impl DoubleEndedIterator<Item = Seq> + '_ {
        self.folded.seqs_from(from)
    }

    pub fn tail_seq(&self) -> Option<Seq> {
        self.folded.tail_seq()
    }

    pub fn len(&self) -> usize {
        self.folded.len()
    }

    pub fn is_empty(&self) -> bool {
        self.folded.is_empty()
    }
}

/// One segment's appended stretch: where this index stopped, and where
/// the segment now ends.
struct Span {
    name: String,
    from: u64,
    size: u64,
}

/// The records of the segment at `path` from `from` to `size`, read
/// forward one window at a time, with the count of bytes lifted off the
/// disk to find them.
///
/// The read is bounded by the length the caller stat'ed (`size`), so a
/// record appended between the stat and the read stays for a later pass
/// rather than being folded at an offset this pass never confirmed. It
/// stops at the window whose records end before the window does: past
/// that point a preallocated segment (storage D31) holds only zeros, so
/// a reader of one takes its records and at most one window of zeros,
/// never every zero up to the roll size (storage D32 and D33). The
/// window starts at `FIRST_WINDOW_BYTES` and doubles, so a long stretch
/// of records costs a number of reads logarithmic in its length.
///
/// The refresh and the full scan both read through here, so the
/// zero-tail rule is `jsonl::records_end` alone and neither caller
/// reads a preallocated segment the other way.
fn read_records(vfs: &dyn Vfs, path: &Path, from: u64, size: u64) -> io::Result<(Vec<u8>, u64)> {
    let mut records = Vec::new();
    let mut read: u64 = 0;
    let mut window = FIRST_WINDOW_BYTES;
    loop {
        let at = from.saturating_add(read);
        let asked = window.min(size.saturating_sub(at));
        if asked == 0 {
            return Ok((records, read));
        }
        let mut chunk = vfs.read_at(path, at, asked)?;
        let lifted = u64::try_from(chunk.len()).map_err(io::Error::other)?;
        read = read.saturating_add(lifted);
        let end = records_end(&chunk);
        let reached_zeros = end < chunk.len();
        chunk.truncate(end);
        records.append(&mut chunk);
        if reached_zeros || lifted < asked {
            return Ok((records, read));
        }
        window = window.saturating_mul(2);
    }
}

/// What a refresh decided before it read anything.
enum RefreshPlan {
    Unchanged,
    Appended { spans: Vec<Span> },
    Rebuild,
}

/// Every line indexed by the rule the index owns.
fn scan(vfs: &dyn Vfs, dir: &Path) -> Result<Folded, StorageError> {
    walk(vfs, dir, |_| Ok(None), |failure| failure)
}

/// The one pass over the segments that builds a `Folded`.
fn walk<E>(
    vfs: &dyn Vfs,
    dir: &Path,
    mut each: impl FnMut(&[u8]) -> Result<Option<Located>, E>,
    lift: impl Fn(StorageError) -> E,
) -> Result<Folded, E> {
    let mut folded = Folded::empty();
    for name in segment_names(vfs, dir).map_err(&lift)? {
        let path = dir.join(&name);
        let size = vfs
            .size(&path)
            .map_err(io_err("stat segment", &path))
            .map_err(&lift)?;
        // The records, not a preallocated segment's zero tail: the same
        // window read and the same stop the refresh takes (storage D33).
        let (bytes, _lifted) = read_records(vfs, &path, 0, size)
            .map_err(io_err("read segment", &path))
            .map_err(&lift)?;
        let mut offset: u64 = 0;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            // A torn tail carries no seq we can trust; it is skipped, and
            // the next append overwrites it (jsonl owns that repair).
            let Some(body) = line.strip_suffix(b"\n") else {
                break;
            };
            if !body.is_empty() {
                match each(body)? {
                    Some(located) => folded.insert_located(&name, offset, located),
                    None => folded.insert_line(&name, offset, body),
                }
            }
            let len = u64::try_from(line.len())
                .map_err(std::io::Error::other)
                .map_err(io_err("measure segment line", &path))
                .map_err(&lift)?;
            offset = offset.saturating_add(len);
        }
        folded.scanned.insert(name, offset);
    }
    Ok(folded)
}

/// Only the folded maps are written: the seam is this process's own, and
/// an index read back reaches disk through a new one. A views snapshot
/// carries the index this way (`crates/storage/spec/Index.lean` §8-4).
impl serde::Serialize for LedgerIndex {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.folded.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for LedgerIndex {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Folded::deserialize(deserializer).map(|folded| LedgerIndex {
            folded,
            vfs: Mutex::new(Box::new(RealFs::new())),
        })
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod scan_tests;
