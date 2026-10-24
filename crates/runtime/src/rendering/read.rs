// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A `read` answer as one header line and the text it carries, with a
//! line number marked every ten lines.
//!
//! The marks are the one grammar `edit` has to see through: a model
//! copies a passage out of what it was shown, marks included, and
//! [`without_marks`] is how `edit` gets the file's own bytes back. The
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
/// one grammar [`without_marks`] takes off again.
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

/// `text` with the line marks [`text`] draws taken off the front of
/// every line that carries one, or `None` when no line carries one.
///
/// A mark is one or more ASCII digits and the separator at the very
/// start of a line. `edit` tries a passage as it came first and asks for
/// this only when that matched nothing, because a file may hold lines
/// that look like marks and an exact match is never second-guessed.
#[must_use]
pub fn without_marks(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    for line in text.split_inclusive('\n') {
        let digits = line.bytes().take_while(u8::is_ascii_digit).count();
        match line
            .get(digits..)
            .and_then(|rest| rest.strip_prefix(MARK_END))
        {
            Some(rest) if digits > 0 => {
                out.push_str(rest);
                changed = true;
            }
            Some(_) | None => out.push_str(line),
        }
    }
    changed.then_some(out)
}
