// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The list half of the person's face of `sprawling view`
//! (`crates/sprawling/spec/Main.lean` §8-117): which tree rows are shown, how each is
//! marked, the window of rows that keeps the cursor on screen, the
//! ledger lines of the records lens on screen, and the list set beside
//! the detail pane.

use std::collections::BTreeSet;

use super::arrange::Entry;
use super::follow::Row;
use super::rounds::{Rounds, leaf_mark};
use super::{HashWidth, chain_label};

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
    let skip = first_shown(cursor, rows);
    lines
        .into_iter()
        .enumerate()
        .skip(skip)
        .take(rows)
        .map(|(at, line)| format!("{}{line}", if at == cursor { '>' } else { ' ' }))
        .collect()
}

/// The records lens on screen: the `rows` lines `scrolled` keeps, each
/// led by the first digits of its chain hash. Only those rows are
/// hashed, so a frame costs what the screen holds, not the ledger.
pub(super) fn record_lines(records: &[Row], cursor: usize, rows: usize) -> Vec<String> {
    let skip = first_shown(cursor, rows);
    let lines = records
        .iter()
        .skip(skip)
        .take(rows)
        .map(|row| {
            format!(
                "{}{}",
                chain_label(row.line.as_bytes(), HashWidth::Glance),
                row.line
            )
        })
        .collect();
    scrolled(lines, cursor.saturating_sub(skip), rows)
}

/// The first line on screen: the one that puts the cursor on the last
/// row once it is below the first screen.
fn first_shown(cursor: usize, rows: usize) -> usize {
    cursor.saturating_add(1).saturating_sub(rows)
}

/// The list on the left half and the detail on the right, split by a
/// column of `|`.
pub(super) fn beside(
    list: &[String],
    detail: &[String],
    columns: usize,
    rows: usize,
) -> Vec<String> {
    let left = columns / 2;
    let right = columns.saturating_sub(left).saturating_sub(1);
    let side = |lines: &[String], at: usize| lines.get(at).map_or("", String::as_str).to_owned();
    (0..rows)
        .map(|at| {
            format!(
                "{:<left$}|{}",
                cut(&side(list, at), left),
                cut(&side(detail, at), right)
            )
        })
        .collect()
}

/// `line` cut to `width` characters.
pub(super) fn cut(line: &str, width: usize) -> String {
    line.chars().take(width).collect()
}
