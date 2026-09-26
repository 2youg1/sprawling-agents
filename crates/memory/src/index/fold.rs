// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The folded index: seq to (segment, byte offset), and the scans that
//! build it.
//!
//! Nothing here is persisted. The maps are rebuilt from the segments
//! whenever a reader opens them, and [`crate::index::LedgerIndex`] keeps
//! them for the life of a process so a query costs one directory listing
//! plus the bytes appended since the last look. A side artifact on disk
//! used to carry the same maps; it had no writer left, and a reader of a
//! file nobody writes is a second answer waiting to disagree.
//!
//! The storage is three parallel columns over lines and a span table
//! over runs ([`Entries`], [`RunTable`]). The public queries are the
//! contract; the layout is theirs to change, and `tests` holds the
//! BTreeMap-shaped oracle that pins the answers.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

use kernel::{RunId, Seq};

use crate::error::{MemoryError, io_err};
use crate::jsonl::segment_names;
use crate::vfs::Vfs;

pub(crate) struct Folded {
    pub(crate) scanned: BTreeMap<String, u64>,
    entries: Entries,
    runs: RunTable,
}

impl Folded {
    pub(crate) fn empty() -> Folded {
        Folded {
            scanned: BTreeMap::new(),
            entries: Entries::empty(),
            runs: RunTable::empty(),
        }
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
    ///
    /// The seq column and the run span table are written here and only
    /// here, so the two cannot tell different stories about who wrote a
    /// line: the seq join that carries a run's name is registered with
    /// its spans in the same call.
    pub(crate) fn insert_line(&mut self, name: &str, offset: u64, body: &[u8]) {
        let Some(located) = locate(body) else {
            return;
        };
        self.entries.insert(located.seq, name, offset);
        if let Some(run) = located.run {
            self.runs.add(run, located.seq);
        }
    }

    /// Where one line sits, and under which segment name.
    pub(crate) fn loc_of(&self, seq: Seq) -> Option<(&str, u64)> {
        self.entries.loc_of(seq)
    }

    /// The seq values one run wrote, newest first, below `before`.
    pub(crate) fn run_seqs_before(&self, run: RunId, before: Option<Seq>) -> Vec<Seq> {
        self.runs.seqs_before(run, before)
    }

    pub(crate) fn tail_seq(&self) -> Option<Seq> {
        self.entries.tail_seq()
    }

    pub(crate) fn seqs(&self) -> &[Seq] {
        &self.entries.seqs
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }
}

/// seq → (segment name, line offset) as three parallel columns.
///
/// `seqs` is strictly increasing and carries the binary search;
/// `locs` rides beside it as (segment dictionary id, byte offset), so a
/// ledger of fifty thousand lines holds fifty thousand offsets and one
/// name per segment instead of one `String` per line. A lookup is one
/// binary search; an append lands at the end of every column, which is
/// the common case by the ledger's own append order.
///
/// Lines arrive in seq order, but a damaged ledger may say otherwise
/// and an index over a damaged ledger is exactly what a repair path
/// needs: an out-of-order line is placed by the same search rather than
/// trusted, and a seq written twice keeps the last location — the map
/// this replaced overwrote the same way.
struct Entries {
    seqs: Vec<Seq>,
    segs: Vec<String>,
    locs: Vec<(usize, u64)>,
}

impl Entries {
    fn empty() -> Entries {
        Entries {
            seqs: Vec::new(),
            segs: Vec::new(),
            locs: Vec::new(),
        }
    }

    fn insert(&mut self, seq: Seq, name: &str, offset: u64) {
        let seg = self.seg_id(name);
        match self.seqs.binary_search(&seq) {
            Ok(at) => {
                if let Some(loc) = self.locs.get_mut(at) {
                    *loc = (seg, offset);
                }
            }
            Err(at) => {
                self.seqs.insert(at, seq);
                self.locs.insert(at, (seg, offset));
            }
        }
    }

    /// The dictionary id of one segment name, minted at first sight.
    fn seg_id(&mut self, name: &str) -> usize {
        if let Some(id) = self.segs.iter().position(|held| held == name) {
            return id;
        }
        let id = self.segs.len();
        self.segs.push(name.to_owned());
        id
    }

    fn loc_of(&self, seq: Seq) -> Option<(&str, u64)> {
        let at = self.seqs.binary_search(&seq).ok()?;
        let (seg, offset) = self.locs.get(at).copied()?;
        let name = self.segs.get(seg)?;
        Some((name.as_str(), offset))
    }

    fn tail_seq(&self) -> Option<Seq> {
        self.seqs.last().copied()
    }

    fn len(&self) -> usize {
        self.seqs.len()
    }

    #[cfg(test)]
    fn resident_bytes(&self) -> usize {
        self.seqs.len() * size_of::<Seq>() + self.locs.len() * size_of::<(usize, u64)>()
    }
}

/// run → the seq values it wrote, as ascending disjoint spans.
///
/// A run is a burst of contiguous writes, so its seq set collapses into
/// spans of consecutive values: the tail span extends on touch, and
/// truly interleaved runs carry a handful of spans instead of one id per
/// line. Spans hold only values both endpoints were seen at — two spans
/// merge exactly when the gap between them has been written too — so a
/// span never claims a seq nobody wrote.
struct RunTable {
    /// Sorted by run, so one lookup is one binary search.
    runs: Vec<RunSpans>,
}

/// One run's spans: ascending, disjoint, and never adjacent, because
/// adjacent spans merge the moment they touch.
struct RunSpans {
    run: RunId,
    spans: Vec<Span>,
}

/// An inclusive span of seq values, `start..=end`.
#[derive(Clone, Copy)]
struct Span {
    start: Seq,
    end: Seq,
}

impl RunTable {
    fn empty() -> RunTable {
        RunTable { runs: Vec::new() }
    }

    fn add(&mut self, run: RunId, seq: Seq) {
        match self.runs.binary_search_by(|held| held.run.cmp(&run)) {
            Ok(at) => {
                if let Some(held) = self.runs.get_mut(at) {
                    held.add(seq);
                }
            }
            Err(at) => self.runs.insert(
                at,
                RunSpans {
                    run,
                    spans: vec![Span {
                        start: seq,
                        end: seq,
                    }],
                },
            ),
        }
    }

    fn seqs_before(&self, run: RunId, before: Option<Seq>) -> Vec<Seq> {
        let found = self
            .runs
            .binary_search_by(|held| held.run.cmp(&run))
            .ok()
            .and_then(|at| self.runs.get(at));
        let Some(held) = found else {
            return Vec::new();
        };
        let mut newest_first = Vec::new();
        for span in held.spans.iter().rev() {
            let mut at = span.end;
            if let Some(before) = before
                && before <= at
            {
                let Some(below) = before.value().checked_sub(1).map(Seq::new) else {
                    // `before` is the first seq there is: nothing
                    // sits below it, and no earlier span can either.
                    return newest_first;
                };
                at = below;
            }
            loop {
                if at < span.start {
                    break;
                }
                newest_first.push(at);
                let Some(below) = at.value().checked_sub(1).map(Seq::new) else {
                    break;
                };
                at = below;
            }
        }
        newest_first
    }
}

impl RunSpans {
    fn add(&mut self, seq: Seq) {
        let at = match self.spans.binary_search_by(|span| span.place(seq)) {
            Ok(_) => return, // the span already covers this value
            Err(at) => at,
        };
        let joins_below = self.spans.get(at).is_some_and(|span| span.abuts_after(seq));
        let joins_above = at
            .checked_sub(1)
            .and_then(|at| self.spans.get(at))
            .is_some_and(|span| span.abuts_before(seq));
        match (joins_above, joins_below) {
            (true, true) => {
                let stretch = self.spans.get(at).map(|span| span.end);
                let reach = at.checked_sub(1).and_then(|at| self.spans.get_mut(at));
                if let (Some(stretch), Some(reach)) = (stretch, reach) {
                    reach.end = stretch;
                }
                self.spans.remove(at);
            }
            (true, false) => {
                if let Some(reach) = at.checked_sub(1).and_then(|at| self.spans.get_mut(at)) {
                    reach.end = seq;
                }
            }
            (false, true) => {
                if let Some(reach) = self.spans.get_mut(at) {
                    reach.start = seq;
                }
            }
            (false, false) => self.spans.insert(
                at,
                Span {
                    start: seq,
                    end: seq,
                },
            ),
        }
    }
}

impl Span {
    /// Where this span sits around `seq`, as `binary_search_by` wants
    /// it: the ordering of the element (this span) against the target.
    fn place(&self, seq: Seq) -> Ordering {
        if seq < self.start {
            Ordering::Greater
        } else if self.end < seq {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }

    /// True when `seq` is the value right below this span, so writing
    /// it stretches the span up one without leaving a hole.
    fn abuts_before(&self, seq: Seq) -> bool {
        self.end.value().checked_add(1) == Some(seq.value())
    }

    /// True when `seq` is the value right above this span.
    fn abuts_after(&self, seq: Seq) -> bool {
        seq.value().checked_add(1) == Some(self.start.value())
    }
}

pub(crate) fn rebuild(vfs: &dyn Vfs, dir: &Path) -> Result<Folded, MemoryError> {
    let mut folded = Folded::empty();
    for name in segment_names(vfs, dir)? {
        let path = dir.join(&name);
        let bytes = vfs.read(&path).map_err(io_err("read segment", &path))?;
        let mut offset: u64 = 0;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            let complete = line.last().copied() == Some(b'\n');
            let body = if complete {
                line.get(..line.len().saturating_sub(1)).unwrap_or(line)
            } else {
                line
            };
            // A torn tail carries no seq we can trust; it is skipped, and
            // the next append overwrites it (jsonl owns that repair).
            if complete && !body.is_empty() {
                folded.insert_line(&name, offset, body);
            }
            if complete {
                folded.scanned.insert(
                    name.clone(),
                    offset.saturating_add(u64::try_from(line.len()).unwrap_or(0)),
                );
            }
            offset = offset.saturating_add(u64::try_from(line.len()).unwrap_or(0));
        }
        folded.scanned.entry(name).or_insert(0);
    }
    Ok(folded)
}

/// Where a line sits and whose it is.
pub(crate) struct Located {
    pub(crate) seq: Seq,
    /// `None` when the line names no run that reads back as one. The
    /// line is still indexed by seq: an index over a damaged ledger is
    /// exactly what a repair path needs, and a line dropped here would
    /// be invisible to every reader.
    pub(crate) run: Option<RunId>,
}

/// Reads two fields off one parse. Indexing must not depend on the
/// record parsing cleanly as an `EventRecord`, so this asks the raw
/// document rather than the typed one.
pub(crate) fn locate(line: &[u8]) -> Option<Located> {
    let value: serde_json::Value = serde_json::from_slice(line).ok()?;
    let seq = Seq::new(value.get("seq")?.as_u64()?);
    let run = value
        .get("run")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| RunId::parse(raw).ok());
    Some(Located { seq, run })
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
