// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading a document as spans, so the interface can show its shape.
//!
//! The one place this city puts a file on screen is a building's own
//! pages, and every one of them is Markdown: `BUILDING.md` and whatever
//! `*.md` sits beside it. Those are what an agent writes for the next
//! agent and what a person reads to find out what happened, and they
//! arrived as one flat wall of `<pre>`.
//!
//! **Spans, not markup.** This returns where things are and what they
//! are; it never rewrites the text. A highlighter that emitted markup
//! would be deciding presentation in the crate that is not allowed to
//! know about presentation, and the interface could no longer choose to
//! show the same document as plain bytes.
//!
//! **No grammar engine here.** Inside a fence the whole block is one
//! `Code` span, because reading `**x**` in a line of Rust as bold text
//! would be this module inventing a fact. When a real grammar is worth
//! its dependency it goes to the server and the wire grows then; the
//! seam is this function's signature.

use serde::{Deserialize, Serialize};

/// What a stretch of a document is.
///
/// Nine, and closed: every arm is something a reader can act on in a
/// document an agent wrote. A tenth needs a reason on screen, not just a
/// pattern in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Token {
    /// A `#` line, hashes included.
    Heading,
    /// `**strong**`.
    Strong,
    /// `*emphasis*` or `_emphasis_`.
    Emphasis,
    /// A `` `span` ``, or everything inside a fence.
    Code,
    /// The ``` line that opens or closes a fence.
    Fence,
    /// The language written after an opening fence.
    Meta,
    /// The `[text](target)` of a link, whole.
    Link,
    /// A list bullet or a numbered marker, at the start of its line.
    Marker,
    /// A `>` quote marker.
    Quote,
}

/// Where one token sits, in bytes from the start of the document.
///
/// Bytes rather than characters because the caller slices the same
/// `&str`, and both ends are always on a character boundary — a document
/// in Chinese would otherwise take the interface down on its first
/// heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub len: u32,
    pub token: Token,
}

/// The fence delimiter, in both spellings a document may use.
const FENCES: [&str; 2] = ["```", "~~~"];

/// Reads a Markdown document as spans, in order and never overlapping.
///
/// Ordered by `start` and disjoint, so the caller walks them once beside
/// the text and never has to decide which of two claims on the same byte
/// wins — that decision is a lexical rule and it belongs here.
#[must_use]
pub fn markdown(text: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut fenced: Option<usize> = None;
    let mut at: usize = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        let indent = trimmed.len().saturating_sub(trimmed.trim_start().len());
        let body = trimmed.trim_start();
        match (fenced, opens_fence(body)) {
            // Closing a fence: the block between the two delimiters is
            // one Code span, and the delimiter line is its own.
            (Some(opened), true) => {
                push(&mut spans, opened, at.saturating_sub(opened), Token::Code);
                push(
                    &mut spans,
                    at.saturating_add(indent),
                    body.len(),
                    Token::Fence,
                );
                fenced = None;
            }
            // Opening one: the delimiter, then the language if it says.
            (None, true) => {
                let start = at.saturating_add(indent);
                let mark = fence_len(body);
                push(&mut spans, start, mark, Token::Fence);
                let info = body.get(mark..).unwrap_or_default();
                let lead = info.len().saturating_sub(info.trim_start().len());
                let named = info.trim();
                if !named.is_empty() {
                    push(
                        &mut spans,
                        start.saturating_add(mark).saturating_add(lead),
                        named.len(),
                        Token::Meta,
                    );
                }
                fenced = Some(at.saturating_add(line.len()));
            }
            // Inside a fence nothing is read: the block is code, and the
            // span for it is pushed when the fence closes.
            (Some(_), false) => {}
            (None, false) => read_line(&mut spans, at, line, trimmed),
        }
        at = at.saturating_add(line.len());
    }
    // A fence nobody closed still ends somewhere, and the text after it
    // is still code. Dropping it would leave the tail of a truncated
    // document rendered as prose.
    if let Some(opened) = fenced {
        push(&mut spans, opened, at.saturating_sub(opened), Token::Code);
    }
    spans.sort_by_key(|span| span.start);
    spans
}

/// One line outside any fence.
fn read_line(spans: &mut Vec<Span>, at: usize, line: &str, trimmed: &str) {
    let indent = trimmed.len().saturating_sub(trimmed.trim_start().len());
    let body = trimmed.trim_start();
    let start = at.saturating_add(indent);
    if body.starts_with('#') {
        push(spans, start, body.len(), Token::Heading);
        return;
    }
    if let Some(rest) = body.strip_prefix('>') {
        push(
            spans,
            start,
            body.len().saturating_sub(rest.len()),
            Token::Quote,
        );
        inline(spans, start.saturating_add(1), rest);
        return;
    }
    if let Some(mark) = marker_len(body) {
        push(spans, start, mark, Token::Marker);
        inline(
            spans,
            start.saturating_add(mark),
            body.get(mark..).unwrap_or_default(),
        );
        return;
    }
    let _ = line;
    inline(spans, start, body);
}

/// How long the list marker at the front of `body` is, when there is one.
///
/// A bullet needs the space after it: `*text*` opens emphasis and `* text`
/// opens a list, and reading the first as a bullet would eat the star a
/// reader meant as punctuation.
fn marker_len(body: &str) -> Option<usize> {
    for bullet in ['-', '*', '+'] {
        if body.starts_with(bullet) && body.get(1..2) == Some(" ") {
            return Some(2);
        }
    }
    let digits = body
        .char_indices()
        .take_while(|&(_, ch)| ch.is_ascii_digit())
        .count();
    if digits == 0 {
        return None;
    }
    let after = body.get(digits..)?;
    if after.starts_with(". ") || after.starts_with(") ") {
        return Some(digits.saturating_add(2));
    }
    None
}

/// Whether this line is a fence delimiter.
fn opens_fence(body: &str) -> bool {
    FENCES.iter().any(|mark| body.starts_with(mark))
}

/// How many bytes of the delimiter this line spells.
fn fence_len(body: &str) -> usize {
    FENCES
        .iter()
        .find(|mark| body.starts_with(**mark))
        .map_or(0, |mark| mark.len())
}

/// The spans inside one line of prose.
///
/// Code first, because a backtick outranks the rest: `` `**x**` `` is a
/// code span containing stars, not bold text inside code. That order is
/// the whole of the precedence this module has, and it lives here rather
/// than in whatever draws the result.
fn inline(spans: &mut Vec<Span>, at: usize, body: &str) {
    let mut taken: Vec<(usize, usize)> = Vec::new();
    scan(spans, &mut taken, at, body, "`", Token::Code);
    scan(spans, &mut taken, at, body, "**", Token::Strong);
    scan(spans, &mut taken, at, body, "*", Token::Emphasis);
    scan(spans, &mut taken, at, body, "_", Token::Emphasis);
    links(spans, &mut taken, at, body);
}

/// Finds every `mark … mark` pair not already claimed.
fn scan(
    spans: &mut Vec<Span>,
    taken: &mut Vec<(usize, usize)>,
    at: usize,
    body: &str,
    mark: &str,
    token: Token,
) {
    let mut from = 0usize;
    while let Some(open) = body.get(from..).and_then(|rest| rest.find(mark)) {
        let start = from.saturating_add(open);
        let after = start.saturating_add(mark.len());
        let Some(close) = body.get(after..).and_then(|rest| rest.find(mark)) else {
            return;
        };
        let end = after.saturating_add(close).saturating_add(mark.len());
        // An empty pair is punctuation somebody typed, not a span.
        if close > 0 && claim(taken, start, end) {
            push(
                spans,
                at.saturating_add(start),
                end.saturating_sub(start),
                token,
            );
        }
        from = end;
    }
}

/// Finds `[text](target)` pairs not already claimed.
fn links(spans: &mut Vec<Span>, taken: &mut Vec<(usize, usize)>, at: usize, body: &str) {
    let mut from = 0usize;
    while let Some(open) = body.get(from..).and_then(|rest| rest.find('[')) {
        let start = from.saturating_add(open);
        let Some(shut) = body.get(start..).and_then(|rest| rest.find("](")) else {
            return;
        };
        let target = start.saturating_add(shut).saturating_add(2);
        let Some(end) = body.get(target..).and_then(|rest| rest.find(')')) else {
            return;
        };
        let stop = target.saturating_add(end).saturating_add(1);
        if claim(taken, start, stop) {
            push(
                spans,
                at.saturating_add(start),
                stop.saturating_sub(start),
                Token::Link,
            );
        }
        from = stop;
    }
}

/// Takes a stretch, or reports that something already has it.
fn claim(taken: &mut Vec<(usize, usize)>, start: usize, end: usize) -> bool {
    if taken.iter().any(|&(from, to)| start < to && from < end) {
        return false;
    }
    taken.push((start, end));
    true
}

/// Records one span, dropping a length that will not fit the wire's
/// width rather than truncating it into a slice that ends mid-character.
fn push(spans: &mut Vec<Span>, start: usize, len: usize, token: Token) {
    if len == 0 {
        return;
    }
    let (Ok(start), Ok(len)) = (u32::try_from(start), u32::try_from(len)) else {
        return;
    };
    spans.push(Span { start, len, token });
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
