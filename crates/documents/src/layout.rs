// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
// Portions copyright (c) 2026 2youg1 and the RefRain contributors

//! Where a document's blocks lie in its bytes (`crates/documents/Spec.lean`
//! D6), migrated from RefRain's `source_layout`.
//!
//! The source stays bytes. A layout only records which byte intervals are
//! blocks; gaps, line endings, indentation and every byte between blocks
//! stay where they are, so blocks and gaps together reproduce the source
//! byte for byte. Only the scanning rules came across: RefRain kept a
//! digest beside the intervals so a layout could not slice other bytes,
//! and here a layout never outlives the bytes it was read from.

use crate::format::Format;
use crate::span::{Span, offset};

/// The block intervals of `source` under `format`'s rule, in order.
///
/// Reads bytes, never characters: the rules look for `\n`, `\r`,
/// spaces, tabs and the two delimiter bytes, which mean the same in
/// every encoding that keeps ASCII as itself, and the caller reads a
/// layout only for such an encoding.
pub(crate) fn blocks(format: Format, source: &[u8]) -> Vec<Span> {
    match format {
        Format::Markdown => markdown_blocks(source),
        Format::Plain => plain_blocks(source),
    }
}

/// Where the blocks that can no longer change end, in Markdown still
/// being written (`crates/documents/Spec.lean` D31, `closedUpTo`).
///
/// Just past the last complete line that is blank outside a code block,
/// is an ATX heading at the margin, or closes a code block opened at the
/// margin. The line still being written closes nothing, because what
/// follows may join it; an indented fence or heading may sit inside a
/// list item or a quote that the next line continues. Fences are read by
/// the rule [`blocks`] reads them by.
pub(crate) fn closed(source: &[u8]) -> u64 {
    let mut closed = 0_u64;
    let mut code: Option<(Delimiter, Margin)> = None;
    for (line, end) in complete_lines(source) {
        let (next, closes) = match (code, Delimiter::of(line.content)) {
            (None, Some(opened)) => (Some((opened, Margin::of(line.content))), false),
            (None, None) => (None, line.is_blank() || line.is_margin_heading()),
            (Some((opened, margin)), Some(closing)) if closing.closes(opened) => {
                (None, margin == Margin::At)
            }
            (Some(open), Some(_) | None) => (Some(open), false),
        };
        code = next;
        if closes {
            closed = end;
        }
    }
    closed
}

/// Where a code block's opening line began: at the margin, or indented
/// one to three columns, where it may belong to a list item or a quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Margin {
    At,
    Indented,
}

impl Margin {
    fn of(line: &[u8]) -> Margin {
        match line.first() {
            Some(b'`' | b'~') => Margin::At,
            Some(_) | None => Margin::Indented,
        }
    }
}

/// Every line that ends with a `\n`, and the offset just past it; the
/// last line, when nothing ends it yet, is left out.
fn complete_lines(source: &[u8]) -> impl Iterator<Item = (Line<'_>, u64)> {
    source
        .split_inclusive(|byte| *byte == b'\n')
        .filter_map(|raw| raw.strip_suffix(b"\n"))
        .scan(0_u64, |start, raw| {
            let line = Line {
                start: *start,
                content: raw.strip_suffix(b"\r").unwrap_or(raw),
            };
            *start = start.saturating_add(offset(raw.len())).saturating_add(1);
            Some((line, *start))
        })
}

/// One line of the source: where it starts, and its content up to but
/// not including its `\r\n` or `\n`.
struct Line<'a> {
    start: u64,
    content: &'a [u8],
}

impl Line<'_> {
    fn content_end(&self) -> u64 {
        self.start.saturating_add(offset(self.content.len()))
    }

    fn is_blank(&self) -> bool {
        self.content.iter().all(u8::is_ascii_whitespace)
    }

    /// An ATX heading opened in the first column: one to six `#`, then a
    /// space, a tab or the end of the line (CommonMark 0.31.2 section 4.2).
    fn is_margin_heading(&self) -> bool {
        let marks = self
            .content
            .iter()
            .take_while(|byte| **byte == b'#')
            .count();
        (1..=6).contains(&marks)
            && self
                .content
                .get(marks)
                .is_none_or(|byte| matches!(byte, b' ' | b'\t'))
    }
}

/// Every line, including the empty one after a final newline.
fn lines(source: &[u8]) -> impl Iterator<Item = Line<'_>> {
    source
        .split(|byte| *byte == b'\n')
        .scan(0_u64, |start, raw| {
            let line = Line {
                start: *start,
                content: raw.strip_suffix(b"\r").unwrap_or(raw),
            };
            *start = start.saturating_add(offset(raw.len())).saturating_add(1);
            Some(line)
        })
}

/// Blank lines separate blocks; a code block between two delimiter lines
/// holds its blank lines.
fn markdown_blocks(source: &[u8]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut open: Option<u64> = None;
    let mut last_end = 0_u64;
    let mut code: Option<Delimiter> = None;
    for line in lines(source) {
        code = match (code, Delimiter::of(line.content)) {
            (None, marker) => marker,
            (Some(opened), Some(closing)) if closing.closes(opened) => None,
            (Some(opened), Some(_) | None) => Some(opened),
        };
        if code.is_none() && line.is_blank() {
            if let Some(start) = open.take() {
                spans.push(Span::ordered(start, last_end));
            }
        } else {
            open.get_or_insert(line.start);
            last_end = line.content_end();
        }
    }
    if let Some(start) = open {
        spans.push(Span::ordered(start, last_end));
    }
    spans
}

/// Every line one block, empty ones included. A block holds no line
/// ending; those bytes stay in the gaps.
fn plain_blocks(source: &[u8]) -> Vec<Span> {
    lines(source)
        .map(|line| Span::ordered(line.start, line.content_end()))
        .collect()
}

/// A line that opens or closes a code block: three or more of one
/// delimiter byte, indented by fewer than four columns.
#[derive(Debug, Clone, Copy)]
struct Delimiter {
    byte: u8,
    width: usize,
}

impl Delimiter {
    fn of(line: &[u8]) -> Option<Delimiter> {
        let indent = line
            .iter()
            .take(4)
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count();
        if indent == 4 {
            return None;
        }
        let rest = line.get(indent..)?;
        let byte = *rest.first()?;
        if !matches!(byte, b'`' | b'~') {
            return None;
        }
        let width = rest.iter().take_while(|seen| **seen == byte).count();
        (width >= 3).then_some(Delimiter { byte, width })
    }

    /// Whether this line closes the block `opened` began: the same byte,
    /// at least as many of it.
    fn closes(self, opened: Delimiter) -> bool {
        self.byte == opened.byte && self.width >= opened.width
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
