// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ledger read from its tail: the newest line first, segment by
//! segment backwards, each segment read from its end through a window
//! that starts at one page and doubles (the rule `first_line` uses), so
//! a reader that wants the last N records pays for their bytes and not
//! for the whole ledger (`crates/storage/spec/Jsonl.lean` §8-1). Each line passes the same
//! judgement a forward reader applies (`LineCheck::judge`); the chain is
//! linked from the newer end instead.

use std::io;
use std::path::{Path, PathBuf};

use kernel::ledger::chain_hash;
use kernel::{B3Hash, GENESIS_PREV, Seq};

use crate::error::{StorageError, io_err};
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

use super::first_line::FIRST_WINDOW_BYTES;
use super::ledger::is_segment;
use super::verify::{CheckedLine, LineCheck, LineFault};

/// One line of the ledger, newest first: its bytes and what it is.
#[derive(Debug, Clone, PartialEq)]
pub struct TailLine {
    pub raw: Vec<u8>,
    pub checked: CheckedLine,
}

/// The ledger's lines from the newest back. The walk ends after the
/// oldest line, or after the first failure it yields.
pub struct TailLines {
    vfs: Box<dyn Vfs>,
    segments: Vec<PathBuf>,
    reading: Option<Backward>,
    newer: Option<Link>,
    ended: bool,
}

/// What the line read before (the newer one) requires of the next.
#[derive(Clone, Copy)]
struct Link {
    seq: Seq,
    prev: B3Hash,
}

/// One segment being read from its end. `held` is the segment's bytes
/// from `unread` up to the end of its last complete line.
struct Backward {
    path: PathBuf,
    unread: u64,
    window: u64,
    held: Vec<u8>,
}

impl TailLines {
    /// The walk over the ledger in `dir`, read only: it never opens the
    /// ledger, never repairs and never writes.
    ///
    /// # Errors
    /// The directory cannot be listed.
    pub fn at(dir: &Path) -> Result<Self, StorageError> {
        Self::through(Box::new(RealFs::new()), dir)
    }

    pub(crate) fn through(vfs: Box<dyn Vfs>, dir: &Path) -> Result<Self, StorageError> {
        let segments = vfs
            .list(dir)
            .map_err(io_err("list ledger dir", dir))?
            .into_iter()
            .filter(|p| is_segment(p))
            .collect();
        Ok(Self {
            vfs,
            segments,
            reading: None,
            newer: None,
            ended: false,
        })
    }

    /// The next non-empty line back, crossing into older segments.
    fn next_raw(&mut self) -> Result<Option<Vec<u8>>, StorageError> {
        loop {
            let reading = match self.reading.as_mut() {
                Some(reading) => reading,
                None => match self.segments.pop() {
                    Some(path) => self
                        .reading
                        .insert(Backward::open(self.vfs.as_ref(), path)?),
                    None => return Ok(None),
                },
            };
            match reading
                .pop_line(self.vfs.as_ref())
                .map_err(io_err("read segment", &reading.path))?
            {
                Some(line) if line.is_empty() => {}
                Some(line) => return Ok(Some(line)),
                None => self.reading = None,
            }
        }
    }

    /// Judges `raw` as a forward reader would and links it to the newer
    /// line: its hash is the newer line's `prev`, its seq the one before.
    fn link(&self, raw: &[u8]) -> Result<(Link, CheckedLine), LineFault> {
        let judged = LineCheck::judge(raw)?;
        if let Some(newer) = self.newer {
            if chain_hash(raw) != newer.prev {
                return Err(LineFault::ChainBreak);
            }
            if judged.seq.next().map_err(LineFault::SeqExhausted)? != newer.seq {
                return Err(LineFault::SeqGap {
                    found: judged.seq,
                    expected: Seq::new(newer.seq.value().saturating_sub(1)),
                });
            }
        }
        if judged.seq == Seq::FIRST && judged.prev != GENESIS_PREV {
            return Err(LineFault::ChainBreak);
        }
        let link = Link {
            seq: judged.seq,
            prev: judged.prev,
        };
        Ok((link, judged.checked))
    }

    /// A refused line, named by the 1-based line the chain places it
    /// at: the newer line's seq. The newest line has no newer one, and
    /// is named line 0.
    fn refusal(&self, fault: LineFault) -> StorageError {
        let line = self.newer.map_or(0, |newer| newer.seq.value());
        StorageError::Envelope {
            path: self
                .reading
                .as_ref()
                .map(|reading| reading.path.clone())
                .unwrap_or_default(),
            line,
            source: fault.into_ax(line),
        }
    }
}

impl Iterator for TailLines {
    type Item = Result<TailLine, StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ended {
            return None;
        }
        let raw = match self.next_raw() {
            Ok(Some(raw)) => raw,
            Ok(None) => return None,
            Err(e) => {
                self.ended = true;
                return Some(Err(e));
            }
        };
        match self.link(&raw) {
            Ok((link, checked)) => {
                self.newer = Some(link);
                Some(Ok(TailLine { raw, checked }))
            }
            Err(fault) => {
                self.ended = true;
                Some(Err(self.refusal(fault)))
            }
        }
    }
}
impl Backward {
    /// The segment at `path`, its torn tail (the bytes after its last
    /// `\n`) already set aside: they are not a line.
    fn open(vfs: &dyn Vfs, path: PathBuf) -> Result<Self, StorageError> {
        let unread = vfs.size(&path).map_err(io_err("size segment", &path))?;
        let mut backward = Self {
            path,
            unread,
            window: FIRST_WINDOW_BYTES,
            held: Vec::new(),
        };
        loop {
            if let Some(end) = backward.held.iter().rposition(|byte| *byte == b'\n') {
                backward.held.truncate(end.saturating_add(1));
                return Ok(backward);
            }
            if backward.unread == 0 {
                backward.held.clear();
                return Ok(backward);
            }
            backward
                .read_further(vfs)
                .map_err(io_err("read segment", &backward.path))?;
        }
    }

    /// The last line `held` ends with, without its `\n`; `None` once the
    /// segment has no line left.
    fn pop_line(&mut self, vfs: &dyn Vfs) -> io::Result<Option<Vec<u8>>> {
        loop {
            let Some((_newline, body)) = self.held.split_last() else {
                return Ok(None);
            };
            let start = match body.iter().rposition(|byte| *byte == b'\n') {
                Some(newline) => newline.saturating_add(1),
                None if self.unread == 0 => 0,
                None => {
                    self.read_further(vfs)?;
                    continue;
                }
            };
            let line = body.get(start..).map(<[u8]>::to_vec);
            self.held.truncate(start);
            return Ok(line);
        }
    }

    /// Puts the window of bytes before `held` in front of it, and doubles
    /// the window for the next read.
    fn read_further(&mut self, vfs: &dyn Vfs) -> io::Result<()> {
        let take = self.window.min(self.unread);
        let from = self.unread.saturating_sub(take);
        let mut bytes = vfs.read_at(&self.path, from, take)?;
        bytes.append(&mut self.held);
        self.held = bytes;
        self.unread = from;
        self.window = self.window.saturating_mul(2);
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
