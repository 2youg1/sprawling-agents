// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::format::Format;
use crate::span::{Span, offset};

pub(crate) fn blocks(format: Format, source: &[u8]) -> Vec<Span> {
    match format {
        Format::Markdown => markdown_blocks(source),
        Format::Plain => plain_blocks(source),
    }
}

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

fn plain_blocks(source: &[u8]) -> Vec<Span> {
    lines(source)
        .map(|line| Span::ordered(line.start, line.content_end()))
        .collect()
}

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
