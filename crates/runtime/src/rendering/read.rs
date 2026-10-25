// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A `read` answer as one header line and the text it carries, with a
//! line number marked every ten lines.
//!
//! The marks are the one grammar `edit` has to see through: a model
//! copies a passage out of what it was shown, marks included, and
//! [`Copied`] is how `edit` gets the file's own bytes back. The
//! grammar is written here once, so the mark drawn and the mark removed
//! cannot drift apart.

use kernel::Payload;
use kernel::event::record::Rendering;
use serde_json::Value;

/// Every line whose number is a multiple of this is marked, and so is
/// the first line of an answer, so a reader never counts more than this
/// many lines to name one.
const MARK_EVERY: u64 = 10;

/// How a renderer numbers the lines it draws. The ledger keeps 0-based
/// indexes under every version; the number a model reads is the index
/// plus the base (`crates/runtime/spec/Rendering.lean` D119).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numbering {
    /// The line at index 0 is line 0 (V1 and V2).
    FromZero,
    /// The line at index 0 is line 1, as Claude Code and a compiler
    /// count (V3).
    FromOne,
}

impl Numbering {
    /// The numbering a renderer draws with.
    #[must_use]
    pub fn of(rendering: Rendering) -> Numbering {
        match rendering {
            Rendering::V1 | Rendering::V2 => Numbering::FromZero,
            Rendering::V3 => Numbering::FromOne,
        }
    }

    /// The number the line at `index` is drawn with; `None` only past
    /// the largest number.
    pub(crate) fn shown(self, index: u64) -> Option<u64> {
        match self {
            Numbering::FromZero => Some(index),
            Numbering::FromOne => index.checked_add(1),
        }
    }

    /// The index of the line drawn with `number`.
    fn index(self, number: u64) -> Option<u64> {
        match self {
            Numbering::FromZero => Some(number),
            Numbering::FromOne => number.checked_sub(1),
        }
    }
}

/// What separates a mark from the line it numbers.
const MARK_END: char = '|';

/// The header and the marked text of one `read` payload, or `None` when
/// the payload is not the shape `tools::read` writes.
///
/// The payload holds 0-based indexes and `numbering` draws them; the
/// interval is closed, and the numbers drawn are the ones `offset` and
/// a `search` hit drawn by the same version take, so a number read off
/// the header or a mark is one a call can pass back unchanged. A payload
/// without `offset` started at index 0. A payload without `total_lines`
/// reached the end of its text, so the total is the last index shown
/// plus one.
pub(super) fn text(result: &Payload, numbering: Numbering) -> Option<String> {
    let map = result.as_map();
    let path = map.get("path")?.as_str()?;
    let body = map.get("text")?.as_str()?;
    let offset = number(map.get("offset"))?.unwrap_or(0);
    let total = number(map.get("total_lines"))?;
    let next = number(map.get("next_offset"))?;
    let lines: Vec<&str> = body.split_inclusive('\n').collect();
    let shown = u64::try_from(lines.len()).ok()?;
    let mut out = String::with_capacity(
        body.len()
            .saturating_add(lines.len().saturating_mul(2) / 5)
            .saturating_add(160),
    );
    out.push_str(path);
    match shown
        .checked_sub(1)
        .and_then(|span| offset.checked_add(span))
    {
        None => match total {
            Some(0) | None => out.push_str(" is empty"),
            Some(total) => out.push_str(&format!(
                " has {total} lines; offset {} is past the end",
                numbering.shown(offset)?
            )),
        },
        Some(last) => {
            let total = match total {
                Some(total) => total,
                None => last.checked_add(1)?,
            };
            out.push_str(&format!(
                " lines {}-{} of {total}",
                numbering.shown(offset)?,
                numbering.shown(last)?
            ));
            if let Some(next) = next {
                out.push_str(&format!(", next {}", numbering.shown(next)?));
            }
        }
    }
    out.push('\n');
    for (at, line) in (offset..).zip(lines) {
        push_line(&mut out, numbering, (offset, at), line)?;
    }
    Some(out)
}

/// Writes the line at index `at` of a passage whose first line is at
/// index `first`, behind the mark [`is_marked`] places there, in the one
/// grammar [`Copied`] takes off again; `None` only for an index whose
/// number overflows.
pub(super) fn push_line(
    out: &mut String,
    numbering: Numbering,
    (first, at): (u64, u64),
    line: &str,
) -> Option<()> {
    let number = numbering.shown(at)?;
    if is_marked(numbering.shown(first)?, number) {
        out.push_str(&format!("{number}{MARK_END}"));
    }
    out.push_str(line);
    Some(())
}

/// Whether the line numbered `at` of a passage whose first line is
/// numbered `first` carries a mark: the first line does, and so does
/// every line whose number is a multiple of [`MARK_EVERY`]. Drawing and
/// [`Copied`] both ask this, so a mark is taken off exactly where one
/// can have been drawn.
fn is_marked(first: u64, at: u64) -> bool {
    at == first || at.is_multiple_of(MARK_EVERY)
}

/// A key that is absent reads as `Some(None)`; one that is present and
/// not a whole number reads as `None`, which is not a `read` payload.
fn number(value: Option<&Value>) -> Option<Option<u64>> {
    match value {
        None => Some(None),
        Some(value) => value.as_u64().map(Some),
    }
}

/// A passage copied out of a `read` answer, each line with the mark it
/// may carry.
///
/// A line that starts with digits and the separator may be a mark the
/// renderer drew or text the file holds, and the passage alone cannot
/// say which. What can say is where the passage sits: a mark is drawn
/// only on the line whose number it spells under the numbering that drew
/// it, and only where [`is_marked`] places one. [`Copied::drawn_from`]
/// keeps to that rule, and `edit` checks the result against the file
/// before it trusts it.
pub struct Copied<'a> {
    lines: Vec<CopiedLine<'a>>,
    numbering: Numbering,
}

/// One line of a [`Copied`] passage.
struct CopiedLine<'a> {
    /// The line as it came, line end included.
    whole: &'a str,
    /// The number its leading mark spells and the line after the mark,
    /// or `None` when it starts with no mark.
    mark: Option<(u64, &'a str)>,
}

impl<'a> Copied<'a> {
    /// The passage split into lines, each line's mark read as
    /// `numbering` draws marks.
    #[must_use]
    pub fn of(text: &'a str, numbering: Numbering) -> Copied<'a> {
        let lines = text
            .split_inclusive('\n')
            .map(|whole| {
                let digits = whole.bytes().take_while(u8::is_ascii_digit).count();
                let mark = whole
                    .get(..digits)
                    .filter(|spelled| !spelled.is_empty())
                    .and_then(|spelled| spelled.parse::<u64>().ok())
                    .zip(
                        whole
                            .get(digits..)
                            .and_then(|rest| rest.strip_prefix(MARK_END)),
                    );
                CopiedLine { whole, mark }
            })
            .collect();
        Copied { lines, numbering }
    }

    /// Each 0-based file index the passage's first line could sit at for
    /// one of its marks to have been drawn there, in ascending order.
    #[must_use]
    pub fn starts(&self) -> std::collections::BTreeSet<u64> {
        (0_u64..)
            .zip(&self.lines)
            .filter_map(|(at, line)| {
                let (number, _) = line.mark?;
                let first = number
                    .checked_sub(at)
                    .filter(|&first| is_marked(first, number))?;
                self.numbering.index(first)
            })
            .collect()
    }

    /// The passage with the marks taken off that a read answer would
    /// have drawn had the passage's first line been at file index
    /// `start`, and the numbers those marks spelled; `None` when no line
    /// carries such a mark.
    #[must_use]
    pub fn drawn_from(&self, start: u64) -> Option<(String, std::collections::BTreeSet<u64>)> {
        let first = self.numbering.shown(start)?;
        let mut out = String::new();
        let mut taken = std::collections::BTreeSet::new();
        for (at, line) in (0_u64..).zip(&self.lines) {
            let drawn = line.mark.filter(|&(number, _)| {
                first.checked_add(at) == Some(number) && is_marked(first, number)
            });
            match drawn {
                Some((number, rest)) => {
                    out.push_str(rest);
                    taken.insert(number);
                }
                None => out.push_str(line.whole),
            }
        }
        (!taken.is_empty()).then_some((out, taken))
    }

    /// The passage with every mark that spells one of `numbers` taken
    /// off, and every other line as it came: what a replacement holds
    /// once the marks copied into it from the passage it replaces are
    /// gone, while a line it means to write with digits and a separator
    /// in front stays.
    #[must_use]
    pub fn without(&self, numbers: &std::collections::BTreeSet<u64>) -> String {
        self.lines
            .iter()
            .map(|line| match line.mark {
                Some((number, rest)) if numbers.contains(&number) => rest,
                Some(_) | None => line.whole,
            })
            .collect()
    }
}
