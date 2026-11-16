// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::ByteLen;

use crate::elision::{self, Cut, Elided};

mod exchange;

pub(crate) use exchange::Exchange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {

    Prose,

    Code,

    Diff,

    Log,

    Structured,

    Table,

    Markup,

    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {

    Head,

    Ends,

    Tail,

    Sections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shrink {

    Keep,

    Cut(Strategy),

    MustOffload,
}

#[must_use]
pub fn detect(text: &str) -> Content {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return Content::Unknown;
    }
    if trimmed.starts_with("diff --git")
        || trimmed.starts_with("--- ")
        || trimmed.starts_with("@@ ")
    {
        return Content::Diff;
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') || trimmed.starts_with("<?xml") {
        return Content::Structured;
    }
    let head: Vec<&str> = trimmed.lines().take(8).collect();

    if looks_like_markup(&head) {
        return Content::Markup;
    }
    if head.len() >= 2 && head.iter().filter(|line| line.contains('|')).count() >= 2 {
        return Content::Table;
    }
    if head
        .iter()
        .filter(|line| looks_like_a_log_line(line))
        .count()
        >= 2
    {
        return Content::Log;
    }
    if head.iter().any(|line| looks_like_code(line)) {
        return Content::Code;
    }
    if head.iter().any(|line| line.split_whitespace().count() >= 5) {
        return Content::Prose;
    }
    Content::Unknown
}

fn looks_like_markup(head: &[&str]) -> bool {
    if head.iter().any(|line| {
        let line = line.trim_start();
        line.starts_with("\\documentclass") || line.starts_with("\\begin{document}")
    }) {
        return true;
    }
    !head.iter().any(|line| looks_like_code(line))
        && head.iter().filter(|line| is_atx_heading(line)).count() >= 2
}

fn looks_like_code(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("fn ")
        || line.starts_with("def ")
        || line.starts_with("class ")
        || line.starts_with("import ")
        || line.starts_with("use ")
        || line.ends_with('{')
        || line.ends_with(';')
}

fn is_atx_heading(line: &str) -> bool {
    let line = line.trim_start();
    let hashes = line.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&hashes) && line.chars().nth(hashes) == Some(' ')
}

fn looks_like_a_log_line(line: &str) -> bool {
    let line = line.trim_start();
    let starts_numeric = line.chars().next().is_some_and(|c| c.is_ascii_digit());
    starts_numeric
        || line.starts_with('[')
        || line.starts_with("INFO")
        || line.starts_with("WARN")
        || line.starts_with("ERROR")
        || line.starts_with("DEBUG")
}

#[must_use]
pub fn plan(content: Content, size: ByteLen, budget: ByteLen) -> Shrink {
    if size.get() <= budget.get() {
        return Shrink::Keep;
    }
    match content {
        Content::Structured => Shrink::MustOffload,
        Content::Log => Shrink::Cut(Strategy::Tail),
        Content::Code | Content::Diff => Shrink::Cut(Strategy::Ends),
        Content::Prose | Content::Table => Shrink::Cut(Strategy::Head),
        Content::Markup => Shrink::Cut(Strategy::Sections),

        Content::Unknown => Shrink::MustOffload,
    }
}

#[must_use]
pub fn shorten(text: &str, strategy: Strategy, budget: ByteLen) -> Cut {
    let limit = usize::try_from(budget.get()).unwrap_or(usize::MAX);
    let cut = match strategy {
        Strategy::Head => keep_head(text, limit),
        Strategy::Tail => keep_tail(text, limit),
        Strategy::Ends => keep_ends(text, limit),
        Strategy::Sections => keep_sections(text, limit),
    };

    if cut.text.len() >= text.len() {
        return Cut::whole(text);
    }
    cut
}

fn keep_head(text: &str, limit: usize) -> Cut {
    let room = elision::marker_room(text.len());
    let front = elision::boundary_before(text, limit.saturating_sub(room));
    elision::splice(text, front, text.len(), Elided::Tail)
}

fn keep_tail(text: &str, limit: usize) -> Cut {
    let room = elision::marker_room(text.len());
    let keep = limit.saturating_sub(room);
    let back = elision::boundary_after(text, text.len().saturating_sub(keep));
    elision::splice(text, 0, back, Elided::Head)
}

fn keep_ends(text: &str, limit: usize) -> Cut {
    let room = elision::gap_marker_room(text.len());
    let each = limit.saturating_sub(room) / 2;
    let front = elision::boundary_before(text, each);
    let back = elision::boundary_after(text, text.len().saturating_sub(each));
    if back <= front {
        return keep_head(text, limit);
    }
    elision::splice(text, front, back, Elided::Middle)
}

struct Heading<'a> {
    start: usize,
    line: &'a str,
}

fn keep_sections(text: &str, limit: usize) -> Cut {
    let headings = headings_of(text);
    if headings.len() < 2 {
        return keep_head(text, limit);
    }
    let room = elision::gap_marker_room(text.len());
    let Some(available) = limit.checked_sub(room) else {
        return keep_head(text, limit);
    };

    let mut head_end = headings.first().map_or(0, |heading| heading.start);
    if head_end > available {
        return keep_head(text, limit);
    }
    let mut index = 0usize;
    while let Some(heading) = headings.get(index) {
        let section_end = headings
            .get(index.saturating_add(1))
            .map_or(text.len(), |next| next.start)
            .max(heading.start);
        if section_end > available {
            break;
        }
        head_end = section_end;
        index = index.saturating_add(1);
    }

    let mut skeleton = String::new();
    for heading in headings.iter().skip(index) {
        if head_end
            .saturating_add(skeleton.len())
            .saturating_add(heading.line.len())
            > available
        {
            break;
        }
        skeleton.push_str(heading.line);
    }

    let head = elision::Boundary::before(text, head_end).head();
    let dropped = text
        .len()
        .saturating_sub(head.len())
        .saturating_sub(skeleton.len());
    if dropped == 0 {
        return keep_head(text, limit);
    }
    let survivors = head.len().saturating_add(skeleton.len());
    let mut out = String::with_capacity(survivors.saturating_add(room));
    out.push_str(head);
    out.push_str(&elision::gap_marker(ByteLen::new(
        u64::try_from(dropped).unwrap_or(u64::MAX),
    )));
    out.push_str(&skeleton);
    Cut {
        text: out,
        dropped: ByteLen::new(u64::try_from(dropped).unwrap_or(u64::MAX)),
        place: Elided::Middle,
    }
}

fn headings_of(text: &str) -> Vec<Heading<'_>> {
    let mut headings = Vec::new();
    let mut at = 0usize;
    for piece in text.split_inclusive('\n') {
        let start = at;
        let end = at.saturating_add(piece.len());
        if is_section_heading(piece.trim_end_matches(['\n', '\r'])) {
            headings.push(Heading { start, line: piece });
        }
        at = end;
    }
    headings
}

fn is_section_heading(line: &str) -> bool {
    let line = line.trim_start();
    is_atx_heading(line)
        || [
            "\\section",
            "\\subsection",
            "\\chapter",
            "\\part",
            "\\paragraph",
        ]
        .iter()
        .any(|command| line.starts_with(command))
}

#[cfg(test)]
mod tests;
