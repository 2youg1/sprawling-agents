// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether a spine document is still the form it was laid out as.
//!
//! A template that nobody has filled in contributes nothing to a prompt
//! and should not be carried into one, so three questions live here:
//! whether a handoff is still blank, what an untouched plan reads as,
//! and which of a plan's rows are still placeholders. They are one
//! subject - the shape of an unedited file - and they are the only
//! readers of the placeholder spellings.

/// Whether a handoff is still the form it was laid out as.
///
/// The test is the form's own parenthetical guidance: every section of
/// the template carries one, and a session that wrote the file replaced
/// them with what it found.
pub(super) fn is_blank_form(text: &str) -> bool {
    let filled = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with('>')
                && !(trimmed.starts_with('(') && trimmed.ends_with(')'))
        })
        .count();
    filled == 0
}

/// The template's example rows are for a person reading the template. A
/// building that starts with them starts with tasks nobody asked for,
/// and they would count in the denominator.
pub(super) fn empty_roadmap(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if !is_placeholder_row(line) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

pub(super) fn is_placeholder_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    let cells: Vec<&str> = trimmed
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect();
    match (cells.len(), cells.first(), cells.get(1)) {
        (kernel::ROADMAP_COLUMNS, Some(index), Some(item)) => {
            kernel::NodeId::parse(index).is_ok() && item.is_empty()
        }
        _ => false,
    }
}
