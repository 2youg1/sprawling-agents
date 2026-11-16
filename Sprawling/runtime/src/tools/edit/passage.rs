// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const AROUND: usize = 3;

const MOST_LINES: usize = 40;
const MOST_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Opened {

    AroundOld,

    AtHead,
}

pub(super) fn around(text: &str, old: &str) -> (Opened, String) {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let found = old
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .and_then(|first| lines.iter().position(|line| line.contains(first)));
    let (opened, at) = match found {
        Some(at) => (Opened::AroundOld, at),
        None => (Opened::AtHead, 0),
    };
    if lines.is_empty() {
        return (opened, "the file is now empty".to_owned());
    }
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
    let numbering = crate::rendering::Numbering::of(crate::rendering::CURRENT);
    let shown = |index: usize| {
        u64::try_from(index)
            .ok()
            .and_then(|index| numbering.shown(index))
            .map_or_else(|| index.to_string(), |number| number.to_string())
    };
    (
        opened,
        format!(
            "lines {}-{} as the file now reads:\n{body}",
            shown(start),
            shown(last)
        ),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::{Opened, around};

    #[test]
    fn the_window_opens_three_lines_above_where_old_begins() {
        let text: String = (0..20).map(|n| format!("line {n}\n")).collect();
        assert_eq!(
            around(&text, "line 10\nline 11"),
            (
                Opened::AroundOld,
                "lines 8-15 as the file now reads:\nline 7\nline 8\nline 9\nline 10\nline 11\nline 12\nline 13\nline 14\n"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_gone_old_opens_the_window_at_the_head() {
        let text: String = (0..20).map(|n| format!("line {n}\n")).collect();
        assert_eq!(
            around(&text, "gone"),
            (
                Opened::AtHead,
                "lines 1-7 as the file now reads:\nline 0\nline 1\nline 2\nline 3\nline 4\nline 5\nline 6\n"
                    .to_owned()
            )
        );
    }
}
