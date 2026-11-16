// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use comrak::nodes::Sourcepos;

use crate::span::{Span, offset};

pub(super) struct Positions<'t> {
    text: &'t str,

    starts: Vec<u64>,
    at: u64,
}

impl<'t> Positions<'t> {
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

    pub(super) fn span(&self, position: Sourcepos) -> Span {
        let start = self.offset_of(position.start.line, position.start.column.saturating_sub(1));
        let end = self.offset_of(position.end.line, position.end.column);
        Span::ordered(start, end.max(start))
    }

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
