// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
// Portions copyright (c) 2026 2youg1 and the RefRain contributors

//! The sentences of a text with the whitespace around them, and the
//! alignment of two such lists (`crates/documents/Spec.lean` D14, D15).
//!
//! Migrated from RefRain's `manuscript::review` (the sentence cut) and
//! `manuscript::align` (the common-subsequence table). Two things moved:
//! a sentence is compared with its whitespace, so the sentences of either
//! side read back that side byte for byte; and the anchored segmentation
//! RefRain needs for a whole manuscript is not here, because a proposal
//! is at most one window on each side.

use super::{Slice, SliceKind};

/// How many cells the common-subsequence table may hold: `u32` each,
/// sixteen MiB in all (D15). Past it the middle of the two lists is one
/// deletion and one insertion: still exact, only coarser.
pub const ALIGN_CELLS_MAX: usize = 1 << 22;

/// One sentence: the whitespace before it, the sentence, and - for the
/// last sentence of a text only - the whitespace after it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Piece {
    lead: String,
    text: String,
    trail: String,
}

impl Piece {
    fn push(&mut self, character: char) {
        if !character.is_whitespace() {
            self.text.push_str(&self.trail);
            self.trail.clear();
            self.text.push(character);
        } else if self.text.is_empty() {
            self.lead.push(character);
        } else {
            self.trail.push(character);
        }
    }

    fn is_blank(&self) -> bool {
        self.text.is_empty()
    }

    fn into_slice(self, kind: SliceKind) -> Slice {
        Slice {
            kind,
            text: self.text,
            lead: self.lead,
            trail: self.trail,
        }
    }
}

/// Cuts `text` into sentences that read back as `text`, byte for byte.
///
/// A sentence ends at a terminator, taking with it the terminators and
/// closing marks that follow at once. Whitespace after the last
/// sentence is that sentence's trail; a text of whitespace alone is one
/// sentence with an empty `text`.
pub(super) fn pieces(text: &str) -> Vec<Piece> {
    let mut found = Vec::new();
    let mut building = Piece::default();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        building.push(character);
        if is_terminator(character) {
            while let Some(&next) = characters.peek() {
                if !is_terminator(next) && !is_closer(next) {
                    break;
                }
                building.push(next);
                characters.next();
            }
            found.push(std::mem::take(&mut building));
        }
    }
    if building != Piece::default() {
        match found.last_mut() {
            Some(last) if building.is_blank() => last.trail.push_str(&building.lead),
            Some(_) | None => found.push(building),
        }
    }
    found
}

fn is_terminator(character: char) -> bool {
    matches!(character, '。' | '！' | '？' | '…' | '!' | '?' | '.')
}

fn is_closer(character: char) -> bool {
    matches!(
        character,
        '"' | '\'' | '\u{201d}' | '\u{2019}' | ')' | '）' | '」' | '』'
    )
}

/// The two lists as one card: what both hold, what only the first holds,
/// what only the second holds, in reading order.
pub(super) fn align(before: Vec<Piece>, after: Vec<Piece>) -> Vec<Slice> {
    let head = before
        .iter()
        .zip(after.iter())
        .take_while(|(one, other)| one == other)
        .count();
    let tail = before
        .iter()
        .skip(head)
        .rev()
        .zip(after.iter().skip(head).rev())
        .take_while(|(one, other)| one == other)
        .count();
    let middle = |pieces: &[Piece]| -> Vec<Piece> {
        pieces
            .iter()
            .skip(head)
            .take(pieces.len().saturating_sub(head).saturating_sub(tail))
            .cloned()
            .collect()
    };
    let (gone, come) = (middle(&before), middle(&after));
    let mut slices: Vec<Slice> = before
        .iter()
        .take(head)
        .cloned()
        .map(|piece| piece.into_slice(SliceKind::Same))
        .collect();
    slices.extend(middle_slices(gone, come));
    slices.extend(
        before
            .into_iter()
            .skip(head)
            .rev()
            .take(tail)
            .rev()
            .map(|piece| piece.into_slice(SliceKind::Same)),
    );
    slices
}

/// The middle of the two lists, aligned by their longest common
/// subsequence, or deleted then inserted whole past the table's budget.
fn middle_slices(gone: Vec<Piece>, come: Vec<Piece>) -> Vec<Slice> {
    let Some(table) = Table::of(&gone, &come) else {
        return gone
            .into_iter()
            .map(|piece| piece.into_slice(SliceKind::Delete))
            .chain(
                come.into_iter()
                    .map(|piece| piece.into_slice(SliceKind::Insert)),
            )
            .collect();
    };
    let mut slices = Vec::with_capacity(gone.len().saturating_add(come.len()));
    let mut gone = gone.into_iter().enumerate().peekable();
    let mut come = come.into_iter().enumerate().peekable();
    while let (Some((left, one)), Some((right, other))) = (gone.peek(), come.peek()) {
        if one == other {
            slices.push(one.clone().into_slice(SliceKind::Same));
            gone.next();
            come.next();
        } else if table.at(left.saturating_add(1), *right)
            >= table.at(*left, right.saturating_add(1))
        {
            slices.push(one.clone().into_slice(SliceKind::Delete));
            gone.next();
        } else {
            slices.push(other.clone().into_slice(SliceKind::Insert));
            come.next();
        }
    }
    slices.extend(gone.map(|(_, piece)| piece.into_slice(SliceKind::Delete)));
    slices.extend(come.map(|(_, piece)| piece.into_slice(SliceKind::Insert)));
    slices
}

/// The length of the longest common subsequence of every pair of
/// suffixes, `width` columns to a row.
struct Table {
    cells: Vec<u32>,
    width: usize,
}

impl Table {
    /// The table for two lists, or `None` when it would pass
    /// [`ALIGN_CELLS_MAX`].
    fn of(gone: &[Piece], come: &[Piece]) -> Option<Table> {
        let width = come.len();
        let size = gone.len().checked_mul(width)?;
        if size > ALIGN_CELLS_MAX {
            return None;
        }
        let mut table = Table {
            cells: vec![0; size],
            width,
        };
        for (left, one) in gone.iter().enumerate().rev() {
            for (right, other) in come.iter().enumerate().rev() {
                let value = if one == other {
                    table
                        .at(left.saturating_add(1), right.saturating_add(1))
                        .saturating_add(1)
                } else {
                    table
                        .at(left.saturating_add(1), right)
                        .max(table.at(left, right.saturating_add(1)))
                };
                if let Some(cell) = table
                    .cell(left, right)
                    .and_then(|at| table.cells.get_mut(at))
                {
                    *cell = value;
                }
            }
        }
        Some(table)
    }

    /// The common length of the suffixes from `left` and `right`. A
    /// suffix past the end of either list is empty, and an empty suffix
    /// has nothing in common with anything: that is what zero states.
    fn at(&self, left: usize, right: usize) -> u32 {
        self.cell(left, right)
            .and_then(|at| self.cells.get(at))
            .copied()
            .unwrap_or(0)
    }

    fn cell(&self, left: usize, right: usize) -> Option<usize> {
        (right < self.width)
            .then(|| left.checked_mul(self.width)?.checked_add(right))
            .flatten()
    }
}
