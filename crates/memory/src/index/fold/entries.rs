// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! seq → (segment name, line offset), eight bytes per line.
//!
//! The kernel numbers lines one after another, so a line's seq is
//! implied by its place in a column that starts at `base`: the column
//! holds one word per line, the segment dictionary id in the high
//! [`SEG_BITS`] bits and the byte offset in the rest. A ledger of fifty
//! thousand lines holds fifty thousand words and one name per segment.
//!
//! A damaged ledger may say otherwise, and an index over a damaged
//! ledger is exactly what a repair path needs. A seq below `base`, one
//! so far past the end that reaching it would leave the column more
//! holes than `max(lines it holds, MIN_REACH)`, and a location too large
//! to pack all go to `outliers`, an
//! ordered map that answers the same questions. Each seq lives in at
//! most one of the two, and a seq written twice keeps the last location.

use std::collections::BTreeMap;

use kernel::Seq;

/// The bits of a column word that carry the segment dictionary id.
const SEG_BITS: u32 = 16;
/// The bits that carry the byte offset: 256 TiB per segment.
const OFFSET_BITS: u32 = u64::BITS - SEG_BITS;
const OFFSET_MASK: u64 = (1 << OFFSET_BITS) - 1;
/// A slot no line has claimed. No packed word equals it, because
/// [`pack`] refuses the one location that would spell it.
const HOLE: u64 = u64::MAX;
/// The holes a stretched column may hold when it holds fewer lines than
/// this. Past that the bound is the lines it holds, so holes never cost
/// more than the lines do.
const MIN_REACH: usize = 64;

pub(super) struct Entries {
    base: Seq,
    column: Vec<u64>,
    /// How many slots of `column` are not [`HOLE`].
    in_column: usize,
    outliers: BTreeMap<Seq, (usize, u64)>,
    segs: Vec<String>,
}

impl Entries {
    pub(super) fn empty() -> Entries {
        Entries {
            base: Seq::FIRST,
            column: Vec::new(),
            in_column: 0,
            outliers: BTreeMap::new(),
            segs: Vec::new(),
        }
    }

    pub(super) fn insert(&mut self, seq: Seq, name: &str, offset: u64) {
        let seg = self.seg_id(name);
        let slot = self.slot_for(seq);
        match (slot, pack(seg, offset)) {
            (Some(at), Some(word)) => {
                self.outliers.remove(&seq);
                self.set_slot(at, word);
            }
            (slot, _) => {
                if let Some(at) = slot {
                    self.set_slot(at, HOLE);
                }
                self.outliers.insert(seq, (seg, offset));
            }
        }
    }

    /// The column slot `seq` belongs in, stretching the column with
    /// holes to reach it; `None` when it belongs with the outliers.
    fn slot_for(&mut self, seq: Seq) -> Option<usize> {
        if self.column.is_empty() && self.outliers.is_empty() {
            self.base = seq;
        }
        let at = usize::try_from(seq.value().checked_sub(self.base.value())?).ok()?;
        if at < self.column.len() {
            return Some(at);
        }
        let new_len = at.checked_add(1)?;
        let lines = self.in_column.checked_add(1)?;
        let holes = new_len.checked_sub(lines)?;
        if holes > lines.max(MIN_REACH) {
            return None;
        }
        self.column.resize(new_len, HOLE);
        Some(at)
    }

    fn set_slot(&mut self, at: usize, word: u64) {
        let Some(slot) = self.column.get_mut(at) else {
            return;
        };
        match (*slot == HOLE, word == HOLE) {
            (true, false) => self.in_column = self.in_column.saturating_add(1),
            (false, true) => self.in_column = self.in_column.saturating_sub(1),
            (true, true) | (false, false) => {}
        }
        *slot = word;
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

    pub(super) fn loc_of(&self, seq: Seq) -> Option<(&str, u64)> {
        let (seg, offset) = match self.outliers.get(&seq) {
            Some(held) => *held,
            None => {
                let at = seq.value().checked_sub(self.base.value())?;
                let word = *self.column.get(usize::try_from(at).ok()?)?;
                unpack(word)?
            }
        };
        Some((self.segs.get(seg)?.as_str(), offset))
    }

    pub(super) fn tail_seq(&self) -> Option<Seq> {
        self.seqs().next_back()
    }

    pub(super) fn len(&self) -> usize {
        self.in_column.saturating_add(self.outliers.len())
    }

    /// Every seq held, ascending, from both stores.
    pub(super) fn seqs(&self) -> Seqs<'_> {
        Seqs {
            column: Ends::new(ColumnSeqs {
                base: self.base.value(),
                words: self.column.iter().enumerate(),
            }),
            outliers: Ends::new(self.outliers.keys().copied()),
        }
    }

    #[cfg(test)]
    pub(super) fn resident_bytes(&self) -> usize {
        self.column
            .len()
            .saturating_mul(size_of::<u64>())
            .saturating_add(self.outliers.len().saturating_mul(size_of::<(Seq, usize, u64)>()))
    }
}

/// `seg` and `offset` in one column word, or `None` when either does not
/// fit (or the pair would spell [`HOLE`]).
fn pack(seg: usize, offset: u64) -> Option<u64> {
    let seg = u64::try_from(seg).ok()?;
    if seg >> SEG_BITS != 0 || offset & !OFFSET_MASK != 0 {
        return None;
    }
    let word = (seg << OFFSET_BITS) | offset;
    (word != HOLE).then_some(word)
}

fn unpack(word: u64) -> Option<(usize, u64)> {
    if word == HOLE {
        return None;
    }
    Some((
        usize::try_from(word >> OFFSET_BITS).ok()?,
        word & OFFSET_MASK,
    ))
}

/// The seqs the column holds, ascending, holes skipped.
struct ColumnSeqs<'a> {
    base: u64,
    words: std::iter::Enumerate<std::slice::Iter<'a, u64>>,
}

impl ColumnSeqs<'_> {
    fn seq_at(&self, (at, word): (usize, &u64)) -> Option<Option<Seq>> {
        if *word == HOLE {
            return Some(None);
        }
        let at = u64::try_from(at).ok()?;
        Some(Some(Seq::new(self.base.checked_add(at)?)))
    }
}

impl Iterator for ColumnSeqs<'_> {
    type Item = Seq;
    fn next(&mut self) -> Option<Seq> {
        loop {
            let held = self.words.next()?;
            if let Some(seq) = self.seq_at(held)? {
                return Some(seq);
            }
        }
    }
}

impl DoubleEndedIterator for ColumnSeqs<'_> {
    fn next_back(&mut self) -> Option<Seq> {
        loop {
            let held = self.words.next_back()?;
            if let Some(seq) = self.seq_at(held)? {
                return Some(seq);
            }
        }
    }
}

/// One ascending source with its next value from each end looked at
/// but not yet taken. When the inner iterator runs dry, the value held
/// at the other end is the last one left.
struct Ends<I> {
    inner: I,
    front: Option<Seq>,
    back: Option<Seq>,
}

impl<I: DoubleEndedIterator<Item = Seq>> Ends<I> {
    fn new(inner: I) -> Ends<I> {
        Ends {
            inner,
            front: None,
            back: None,
        }
    }

    fn peek_front(&mut self) -> Option<Seq> {
        if self.front.is_none() {
            self.front = self.inner.next().or_else(|| self.back.take());
        }
        self.front
    }

    fn peek_back(&mut self) -> Option<Seq> {
        if self.back.is_none() {
            self.back = self.inner.next_back().or_else(|| self.front.take());
        }
        self.back
    }
}

/// Every seq [`Entries`] holds, ascending: the column and the outliers
/// merged, which is sound because the two hold disjoint seqs.
pub(crate) struct Seqs<'a> {
    column: Ends<ColumnSeqs<'a>>,
    outliers: Ends<std::iter::Copied<std::collections::btree_map::Keys<'a, Seq, (usize, u64)>>>,
}

impl Iterator for Seqs<'_> {
    type Item = Seq;
    fn next(&mut self) -> Option<Seq> {
        match (self.column.peek_front(), self.outliers.peek_front()) {
            (Some(column), Some(outlier)) if outlier < column => self.outliers.front.take(),
            (Some(_), _) => self.column.front.take(),
            (None, _) => self.outliers.front.take(),
        }
    }
}

impl DoubleEndedIterator for Seqs<'_> {
    fn next_back(&mut self) -> Option<Seq> {
        match (self.column.peek_back(), self.outliers.peek_back()) {
            (Some(column), Some(outlier)) if outlier > column => self.outliers.back.take(),
            (Some(_), _) => self.column.back.take(),
            (None, _) => self.outliers.back.take(),
        }
    }
}
