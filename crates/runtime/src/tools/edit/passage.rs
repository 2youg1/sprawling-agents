// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The lines around `old` as the file now reads, shown when the file
//! moved after the run's last look.
//!
//! The passage is a window read: the refusal records the current stamp
//! (`crates/runtime/spec/Tools/Edit.lean` §8-64), so a model that fixes
//! `old` from these lines retries at once instead of reading the whole
//! file again. The lines go back unmarked, the way the file holds them,
//! so they can be copied into `old` as they stand.

/// Lines kept on each side of the passage.
const AROUND: usize = 3;
/// The most lines a passage carries, and the most bytes: a refusal is
/// one line of advice and a window, not a second read of the file.
const MOST_LINES: usize = 40;
const MOST_BYTES: usize = 4096;

/// The window around where `old` begins in `text`, with a header naming
/// its 0-based lines, or `None` when the first line of `old` is no longer
/// anywhere in the file.
pub(super) fn around(text: &str, old: &str) -> Option<String> {
    let first = old.lines().map(str::trim).find(|line| !line.is_empty())?;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let at = lines.iter().position(|line| line.contains(first))?;
    let start = at.saturating_sub(AROUND);
    let reach = old.lines().count().saturating_add(AROUND.saturating_mul(2));
    let end = start.saturating_add(reach.min(MOST_LINES)).min(lines.len());
    let mut body = String::new();
    let mut last = start;
    for (number, line) in lines.iter().enumerate().take(end).skip(start) {
        if body.len().saturating_add(line.len()) > MOST_BYTES && number > at {
            break;
        }
        body.push_str(line);
        last = number;
    }
    if !body.ends_with('\n') {
        body.push('\n');
    }
    Some(format!(
        "lines {start}-{last} as the file now reads:\n{body}"
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::around;

    #[test]
    fn the_window_opens_three_lines_above_where_old_begins() {
        let text: String = (0..20).map(|n| format!("line {n}\n")).collect();
        let shown = around(&text, "line 10\nline 11").unwrap();
        assert_eq!(
            shown,
            "lines 7-14 as the file now reads:\nline 7\nline 8\nline 9\nline 10\nline 11\nline 12\nline 13\nline 14\n"
        );
        assert_eq!(around(&text, "gone"), None);
    }
}
