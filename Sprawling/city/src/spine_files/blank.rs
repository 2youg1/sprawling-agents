// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(super) fn is_blank_form(text: &str) -> bool {
    let filled = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with('>')
                && !trimmed.starts_with('<')
                && !is_guidance(trimmed)
        })
        .count();
    filled == 0
}

pub(crate) fn is_guidance(trimmed: &str) -> bool {
    trimmed.starts_with('(') && trimmed.ends_with(')')
}

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
