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
use serde_json::Value;

/// Every line whose 0-based number is a multiple of this is marked, and
/// so is the first line of an answer, so a reader never counts more
/// than this many lines to name one.
const MARK_EVERY: u64 = 10;

/// What separates a mark from the line it numbers.
const MARK_END: char = '|';

/// The header and the marked text of one `read` payload, or `None` when
/// the payload is not the shape `tools::read` writes.
///
/// Line numbers are 0-based and the interval is closed, the numbering
/// `offset` and a `search` hit already use, so a number read off the
/// header or a mark is one a call can pass back unchanged. A payload
/// without `offset` started at line 0. A payload without `total_lines`
/// reached the end of its text, so the total is the last line shown
/// plus one.
pub(super) fn text(result: &Payload) -> Option<String> {
    let map = result.as_map();
    let path = map.get("path")?.as_str()?;
    let body = map.get("text")?.as_str()?;
    let offset = number(map.get("offset"))?.unwrap_or(0);
    let total = number(map.get("total_lines"))?;
    let next = number(map.get("next_offset"))?;
    let version = match map.get("version") {
        Some(value) => Some(value.as_str()?),
        None => None,
    };
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
                " has {total} lines; offset {offset} is past the end"
            )),
        },
        Some(last) => {
            let total = match total {
                Some(total) => total,
                None => last.checked_add(1)?,
            };
            out.push_str(&format!(" lines {offset}-{last} of {total}"));
            if let Some(next) = next {
                out.push_str(&format!(", next {next}"));
            }
        }
    }
    if let Some(version) = version {
        out.push_str(&format!(", version {version}"));
    }
    out.push('\n');
    for (at, line) in (offset..).zip(lines) {
        if at == offset || at % MARK_EVERY == 0 {
            push_mark(&mut out, at);
        }
        out.push_str(line);
    }
    Some(out)
}

/// Writes the mark that numbers the line about to be written, in the
/// one grammar [`Copied`] takes off again.
pub(super) fn push_mark(out: &mut String, at: u64) {
    out.push_str(&format!("{at}{MARK_END}"));
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
/// only on the line whose 0-based number it spells, and only on the
/// first line of an answer or a line whose number is a multiple of
/// [`MARK_EVERY`]. [`Copied::drawn_from`] keeps to that rule, and `edit`
/// checks the result against the file before it trusts it.
pub struct Copied<'a> {
    lines: Vec<CopiedLine<'a>>,
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
    /// The passage split into lines, each line's mark read.
    #[must_use]
    pub fn of(text: &'a str) -> Copied<'a> {
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
        Copied { lines }
    }

    /// Each 0-based file line the passage's first line could sit on for
    /// one of its marks to have been drawn there, in ascending order.
    #[must_use]
    pub fn starts(&self) -> std::collections::BTreeSet<u64> {
        (0_u64..)
            .zip(&self.lines)
            .filter_map(|(at, line)| {
                let (number, _) = line.mark?;
                (at == 0 || number % MARK_EVERY == 0)
                    .then(|| number.checked_sub(at))
                    .flatten()
            })
            .collect()
    }

    /// The passage with the marks taken off that a read answer would
    /// have drawn had the passage's first line been file line `start`,
    /// and the numbers those marks spelled; `None` when no line carries
    /// such a mark.
    #[must_use]
    pub fn drawn_from(&self, start: u64) -> Option<(String, std::collections::BTreeSet<u64>)> {
        let mut out = String::new();
        let mut taken = std::collections::BTreeSet::new();
        for (at, line) in (0_u64..).zip(&self.lines) {
            let drawn = line.mark.filter(|(number, _)| {
                start.checked_add(at) == Some(*number) && (at == 0 || number % MARK_EVERY == 0)
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
