// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! When to shorten something, and what to keep.
//!
//! The mechanism half of this — moving a big result out of the window
//! and leaving a reference — is `offload`'s and was frozen first. This
//! is the policy half: given a piece of text, what kind of thing is it,
//! and which end of it carries the information.
//!
//! Three rules hold the whole module up.
//!
//! **The result is never longer than the input.** Shortening that
//! lengthens is not a corner case, it is the failure that makes a
//! context budget meaningless, so it is an invariant with a test rather
//! than a property people remember.
//!
//! **No regular expressions on this path.** Detection is a handful of
//! prefix and count checks over the first few lines; a pattern engine
//! here would put backtracking between a model and its next turn.
//!
//! **Cuts land on character boundaries.** A budget in bytes and a cut in
//! the middle of a character is how a compactor produces text nothing
//! downstream can read. The boundary rule and the marker that announces
//! the loss both belong to `elision`, which every shortening path here
//! shares.

use kernel::ByteLen;

use crate::elision::{self, Cut, Elided};

/// What a piece of text is, as far as shortening is concerned. Eight,
/// and `Unknown` is one of them: a compactor that had to guess would
/// guess wrong on exactly the material nobody anticipated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
    /// Sentences. The beginning says what it is about.
    Prose,
    /// Source. Both ends matter and the middle repeats.
    Code,
    /// A patch. The hunk headers are the information.
    Diff,
    /// Lines with a shape, most of them the same. The end is the news.
    Log,
    /// Structured data. Shortening it makes it unparseable, so it goes
    /// out of the window whole instead.
    Structured,
    /// Rows and columns. The header row is load-bearing.
    Table,
    /// A document with a section structure: LaTeX, or Markdown with
    /// headings. Its bones are the headings, so it is cut at section
    /// boundaries and the headings survive the cut.
    Markup,
    /// Anything else, including binary-looking material.
    Unknown,
}

/// Which end of a text survives a shortening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Keep the front. Prose and tables read from the top.
    Head,
    /// Keep both ends and mark the gap. Code and diffs are read from
    /// the edges inward.
    Ends,
    /// Keep the back. A log's news is at the end.
    Tail,
    /// Keep whole sections from the front, then the headings of the
    /// sections that did not fit. The reader loses bodies and keeps the
    /// map of what was written.
    Sections,
}

/// What has to happen to one piece of text: decided here, carried out
/// by the caller.
///
/// Exhaustive, and `MustOffload` is why it exists. This module used to
/// answer "do not shorten it" with a strategy the caller was free to
/// read as "shorten it plainly, then", and the caller did - which left
/// one crate holding two answers to whether structured data may be
/// truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shrink {
    /// It is within budget and goes to the model unchanged.
    Keep,
    /// Shorten it, keeping this end.
    Cut(Strategy),
    /// Do not shorten it at all — move it out of the window and leave a
    /// reference. Truncated structured data is worse than absent
    /// structured data, because it looks like it can be parsed.
    MustOffload,
}

/// Detection, in priority order, over the first lines only.
///
/// Order is the design: a diff is also code, a table is also prose, and
/// whichever check runs first wins. Cheapest and most specific first.
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
    // Markup comes before Table: a LaTeX document's proof tables put two
    // `|` lines in the first screenful, and a table it is not. It comes
    // before Code too, so a `.tex` preamble is not read as source.
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

/// Whether the first lines read as a document rather than as a program.
///
/// LaTeX announces itself with the two commands that open a document.
/// Markdown is recognised by heading density, and only in a text whose
/// first lines hold nothing that reads as code: a source file's `# `
/// comments share the heading shape, and reading them as a skeleton
/// would cut code while keeping its comments.
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

/// One line that reads as source: a declaration keyword, or the pair of
/// braces and semicolons statements end in.
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

/// A Markdown heading: one to six `#`, then a space. A Python comment
/// with no space after its `#`, and a `#!` line, are neither.
fn is_atx_heading(line: &str) -> bool {
    let line = line.trim_start();
    let hashes = line.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&hashes) && line.chars().nth(hashes) == Some(' ')
}

/// A log line is one that starts with something that is not prose: a
/// timestamp, a level, or a bracket. Counted rather than matched, so
/// one stray line does not decide.
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

/// What happens to that kind of content, at that size.
///
/// The size comparison lives here rather than at the call site, so that
/// "does it fit" and "what is done when it does not" are one decision
/// with one home.
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
        // Nothing is known about it, so nothing is thrown away on a
        // guess: it leaves the window whole and keeps its reference.
        Content::Unknown => Shrink::MustOffload,
    }
}

/// Carries out a shortening the caller has already decided on.
///
/// The answer carries the text, the source bytes it lost and which end
/// lost them, so no caller has to recover the loss by subtracting two
/// lengths — a subtraction that counted the inserted marker as surviving
/// text and under-reported the loss by its width.
///
/// Taking the strategy rather than deciding it again is what keeps this
/// module out of the business of what leaves the window: a caller
/// holding [`Shrink::MustOffload`] has nothing to call here.
#[must_use]
pub fn shorten(text: &str, strategy: Strategy, budget: ByteLen) -> Cut {
    let limit = usize::try_from(budget.get()).unwrap_or(usize::MAX);
    let cut = match strategy {
        Strategy::Head => keep_head(text, limit),
        Strategy::Tail => keep_tail(text, limit),
        Strategy::Ends => keep_ends(text, limit),
        Strategy::Sections => keep_sections(text, limit),
    };
    // The invariant, checked here rather than trusted: if a strategy
    // ever produced something longer, the input is returned instead. A
    // compactor that grows its input makes every budget meaningless.
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

/// One heading line and the bytes it occupies, its line terminator
/// included: the skeleton is assembled from source spans only, so the
/// reported loss stays the distance between what was read and what was
/// kept.
struct Heading {
    start: usize,
    end: usize,
}

/// Whole sections from the front while they fit, then the headings of
/// the sections that did not. A reader of a document loses bodies and
/// keeps the map of what was written, which is what a section cut is
/// for. Fewer than two headings is a title rather than a structure, and
/// falls back to keeping the head.
fn keep_sections(text: &str, limit: usize) -> Cut {
    let headings = headings_of(text);
    if headings.len() < 2 {
        return keep_head(text, limit);
    }
    let room = elision::gap_marker_room(text.len());
    let Some(available) = limit.checked_sub(room) else {
        return keep_head(text, limit);
    };
    // Anything before the first heading (a LaTeX preamble, a Markdown
    // title block) travels with the first section.
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
    // Every heading of every dropped section, while it fits: what the
    // strategy exists to keep is this list.
    let mut skeleton = String::new();
    for heading in headings.iter().skip(index) {
        let Some(piece) = text.get(heading.start..heading.end) else {
            continue;
        };
        if head_end
            .saturating_add(skeleton.len())
            .saturating_add(piece.len())
            > available
        {
            break;
        }
        skeleton.push_str(piece);
    }
    let dropped = text
        .len()
        .saturating_sub(head_end)
        .saturating_sub(skeleton.len());
    if dropped == 0 {
        return keep_head(text, limit);
    }
    let mut out =
        String::with_capacity(head_end.saturating_add(room).saturating_add(skeleton.len()));
    out.push_str(text.get(..head_end).unwrap_or_default());
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

/// Every heading line in the text, in order, with the span it occupies
/// including its terminator.
fn headings_of(text: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut at = 0usize;
    for piece in text.split_inclusive('\n') {
        let start = at;
        let end = at.saturating_add(piece.len());
        if is_section_heading(piece.trim_end_matches(['\n', '\r'])) {
            headings.push(Heading { start, end });
        }
        at = end;
    }
    headings
}

/// The lines a section cut recognises as structure: a Markdown heading,
/// or the LaTeX commands that open a section.
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
