// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The list half of the person's face of `sprawling view`
//! (sprawling-SPEC.md 8-91): which tree rows are shown, how each is
//! marked, and the window of rows that keeps the cursor on screen.

use std::collections::BTreeSet;

use super::arrange::Entry;
use super::rounds::{Rounds, leaf_mark};

/// The entries shown: every one whose ancestors are all open.
pub(super) fn visible(entries: &[Entry], expanded: &BTreeSet<usize>) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry
                .parent
                .is_none_or(|parent| is_open(entries, expanded, parent))
        })
        .map(|(at, _)| at)
        .collect()
}

fn is_open(entries: &[Entry], expanded: &BTreeSet<usize>, at: usize) -> bool {
    let mut cursor = Some(at);
    while let Some(at) = cursor {
        if !expanded.contains(&at) {
            return false;
        }
        cursor = entries.get(at).and_then(|entry| entry.parent);
    }
    true
}

pub(super) fn has_children(entries: &[Entry], at: usize) -> bool {
    entries
        .get(at.saturating_add(1))
        .is_some_and(|next| next.parent == Some(at))
}

/// One row per shown entry: indent, then `+` (children, folded), `-`
/// (open) or the leaf mark, then the label.
pub(super) fn tree_lines(
    entries: &[Entry],
    shown: &[usize],
    expanded: &BTreeSet<usize>,
    rounds: &Rounds,
) -> Vec<String> {
    shown
        .iter()
        .filter_map(|at| entries.get(*at).map(|entry| (*at, entry)))
        .map(|(at, entry)| {
            let mark = match (has_children(entries, at), expanded.contains(&at)) {
                (false, _) => leaf_mark(rounds, &entry.key),
                (true, true) => '-',
                (true, false) => '+',
            };
            format!("{}{mark} {}", "  ".repeat(entry.depth), entry.label)
        })
        .collect()
}

/// The `rows` lines that end at the cursor when it is below the first
/// screen, the cursor's line marked `>`.
pub(super) fn scrolled(lines: Vec<String>, cursor: usize, rows: usize) -> Vec<String> {
    let skip = cursor.saturating_add(1).saturating_sub(rows);
    lines
        .into_iter()
        .enumerate()
        .skip(skip)
        .take(rows)
        .map(|(at, line)| format!("{}{line}", if at == cursor { '>' } else { ' ' }))
        .collect()
}

/// `line` cut to `width` characters.
pub(super) fn cut(line: &str, width: usize) -> String {
    line.chars().take(width).collect()
}
