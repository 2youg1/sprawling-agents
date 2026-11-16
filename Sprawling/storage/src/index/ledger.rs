// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refreshed {
    Unchanged,

    Appended { bytes_read: u64 },

    Rebuilt,
}

pub struct LedgerIndex {
    pub(crate) folded: Folded,
    vfs: Mutex<Box<dyn Vfs>>,
}

impl LedgerIndex {
    fn seam(&self) -> MutexGuard<'_, Box<dyn Vfs>> {
        self.vfs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn rebuild(dir: &Path) -> Result<LedgerIndex, StorageError> {
        let vfs: Box<dyn Vfs> = Box::new(RealFs::new());
        let folded = scan(vfs.as_ref(), dir)?;
        Ok(LedgerIndex {
            folded,
            vfs: Mutex::new(vfs),
        })
    }

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

    #[must_use]
    pub fn empty() -> LedgerIndex {
        LedgerIndex {
            folded: Folded::empty(),
            vfs: Mutex::new(Box::new(RealFs::new())),
        }
    }

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

    fn plan_refresh(&self, dir: &Path) -> Result<RefreshPlan, StorageError> {
        let vfs = self.seam();
        let names = crate::jsonl::segment_names(vfs.as_ref(), dir)?;
        let mut spans = Vec::new();

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

    fn fold_spans(&mut self, dir: &Path, spans: Vec<Span>) -> Result<Refreshed, StorageError> {
        let mut bytes_read: u64 = 0;
        for span in spans {
            let path = dir.join(&span.name);
            let (tail, read) = read_records(self.seam().as_ref(), &path, span.from, span.size)
                .map_err(io_err("read segment", &path))?;
            bytes_read = bytes_read.saturating_add(read);
            let indexed = self.folded.fold_segment(&span.name, span.from, &tail);

            self.folded
                .scanned
                .insert(span.name, span.from.saturating_add(indexed).min(span.size));
        }
        Ok(Refreshed::Appended { bytes_read })
    }

    pub fn reader(&self, dir: &Path) -> LineReader<'_> {
        LineReader {
            index: self,
            dir: dir.to_path_buf(),
            open: None,
        }
    }

    pub fn run_seqs_before(&self, run: RunId, before: Option<Seq>) -> impl Iterator<Item = Seq> {
        self.folded.run_seqs_before(run, before)
    }

    pub fn seqs(&self) -> impl DoubleEndedIterator<Item = Seq> + '_ {
        self.folded.seqs()
    }

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

struct Span {
    name: String,
    from: u64,
    size: u64,
}

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

enum RefreshPlan {
    Unchanged,
    Appended { spans: Vec<Span> },
    Rebuild,
}

fn scan(vfs: &dyn Vfs, dir: &Path) -> Result<Folded, StorageError> {
    walk(vfs, dir, |_| Ok(None), |failure| failure)
}

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

        let (bytes, _lifted) = read_records(vfs, &path, 0, size)
            .map_err(io_err("read segment", &path))
            .map_err(&lift)?;
        let mut offset: u64 = 0;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
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
