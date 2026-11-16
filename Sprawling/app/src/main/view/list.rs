// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use super::arrange::Entry;
use super::follow::Row;
use super::rounds::{Rounds, leaf_mark};
use super::{HashWidth, chain_label};

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

fn first_shown(cursor: usize, rows: usize) -> usize {
    cursor.saturating_add(1).saturating_sub(rows)
}

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

pub(super) fn cut(line: &str, width: usize) -> String {
    line.chars().take(width).collect()
}
