// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a node comrak read lies in the version (`crates/documents/Spec.lean`
//! D26).
//!
//! comrak states a position as a line and a column, both from 1, the
//! column counted in UTF-8 bytes and the end column inclusive. A preview
//! states it as a half-open stretch of the version's bytes, so a page
//! can line a block up with its source. The stretch locates; it does not
//! edit, because a source position is not a lossless edit (refrain
//! roadmap appendix F).

use comrak::nodes::Sourcepos;

use crate::span::{Span, offset};

/// The lines of one window's text, and where that text starts in the
/// version.
pub(super) struct Positions<'t> {
    text: &'t str,
    /// Where each line starts in `text`, split where comrak splits: at
    /// `\r\n`, `\r` and `\n`.
    starts: Vec<u64>,
    at: u64,
}

impl<'t> Positions<'t> {
    /// The positions of `text`, whose first byte is byte `at` of the
    /// version.
    pub(super) fn of(text: &'t str, at: u64) -> Positions<'t> {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        let mut index = 0_usize;
        while let Some(byte) = bytes.get(index) {
            let next = index.saturating_add(1);
            let after = match (*byte, bytes.get(next)) {
                (b'\r', Some(b'\n')) => Some(next.saturating_add(1)),
                (b'\r' | b'\n', _) => Some(next),
                _ => None,
            };
            index = after.unwrap_or(next);
            if let Some(start) = after {
                starts.push(offset(start));
            }
        }
        Positions { text, starts, at }
    }

    /// A source position as the version's bytes: the inclusive end
    /// column made exclusive, and anything past the window held to its
    /// end.
    pub(super) fn span(&self, position: Sourcepos) -> Span {
        let start = self.offset_of(position.start.line, position.start.column.saturating_sub(1));
        let end = self.offset_of(position.end.line, position.end.column);
        Span::ordered(start, end.max(start))
    }

    /// The text of `span`; `None` when it is empty or not inside the
    /// window.
    pub(super) fn source(&self, span: Span) -> Option<String> {
        let start = usize::try_from(span.start().checked_sub(self.at)?).ok()?;
        let end = usize::try_from(span.end().checked_sub(self.at)?).ok()?;
        self.text
            .get(start..end)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    }

    fn offset_of(&self, line: usize, column: usize) -> u64 {
        let length = offset(self.text.len());
        let line_start = line
            .checked_sub(1)
            .and_then(|index| self.starts.get(index))
            .copied()
            .unwrap_or(length);
        self.at
            .saturating_add(line_start.saturating_add(offset(column)).min(length))
    }
}
