// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Slice, SliceKind};

pub const ALIGN_CELLS_MAX: usize = 1 << 22;

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

struct Table {
    cells: Vec<u32>,
    width: usize,
}

impl Table {
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
