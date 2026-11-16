// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::Seq;

const SEG_BITS: u32 = 16;

const OFFSET_BITS: u32 = u64::BITS - SEG_BITS;
const OFFSET_MASK: u64 = (1 << OFFSET_BITS) - 1;

const HOLE: u64 = u64::MAX;

const MIN_REACH: usize = 64;

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Entries {
    base: Seq,
    column: Vec<u64>,

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

    pub(super) fn seqs(&self) -> Seqs<'_> {
        self.seqs_from(Seq::FIRST)
    }

    pub(super) fn seqs_from(&self, from: Seq) -> Seqs<'_> {
        let column_len = u64::try_from(self.column.len()).unwrap_or(u64::MAX);
        let skipped = from
            .value()
            .saturating_sub(self.base.value())
            .min(column_len);
        let words = usize::try_from(skipped)
            .ok()
            .and_then(|at| self.column.get(at..))
            .unwrap_or(&[]);
        Seqs {
            column: Ends::new(ColumnSeqs {
                base: self.base.value().saturating_add(skipped),
                words: words.iter().enumerate(),
            }),
            outliers: Ends::new(OutlierSeqs(self.outliers.range(from..))),
        }
    }

    #[cfg(test)]
    pub(super) fn resident_bytes(&self) -> usize {
        self.column
            .len()
            .saturating_mul(size_of::<u64>())
            .saturating_add(
                self.outliers
                    .len()
                    .saturating_mul(size_of::<(Seq, usize, u64)>()),
            )
    }
}

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

struct OutlierSeqs<'a>(std::collections::btree_map::Range<'a, Seq, (usize, u64)>);

impl Iterator for OutlierSeqs<'_> {
    type Item = Seq;
    fn next(&mut self) -> Option<Seq> {
        self.0.next().map(|(seq, _)| *seq)
    }
}

impl DoubleEndedIterator for OutlierSeqs<'_> {
    fn next_back(&mut self) -> Option<Seq> {
        self.0.next_back().map(|(seq, _)| *seq)
    }
}

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

pub(crate) struct Seqs<'a> {
    column: Ends<ColumnSeqs<'a>>,
    outliers: Ends<OutlierSeqs<'a>>,
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
