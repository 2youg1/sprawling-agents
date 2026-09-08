// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The line-level passes, each a pure function from lines to lines.
//! None of them decides whether its own result is accepted: `sieve`
//! measures every pass and keeps it only when it shrank the text.
//!
//! Two rules shared by all of them: a protected line (`scan`) is
//! never rewritten, and every cut lands on a character boundary.

use std::collections::BTreeMap;

use crate::compaction::boundary_before;
use crate::sieve::scan::{Priority, is_protected};

/// Runs of this many blank lines and more fold to one.
pub(crate) const BLANK_RUN_FOLDS_AT: usize = 3;
/// A template seen this often is said once, with its count.
pub(crate) const TEMPLATE_FOLDS_AT: usize = 4;
/// A line longer than this is cut and marked.
pub(crate) const LONG_LINE_BYTES: usize = 2048;

/// What a terminal would have shown: control sequences removed and,
/// within one line, only the text after the last carriage return.
pub(crate) fn strip_ansi(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let shown = line
                .rsplit('\r')
                .find(|segment| !segment.is_empty())
                .unwrap_or("");
            remove_escapes(shown)
        })
        .collect()
}

fn remove_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.next() {
            // CSI: parameters and intermediates, then one final byte.
            Some('[') => {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            // OSC: runs to BEL or to ESC-backslash.
            Some(']') => {
                let mut previous = ' ';
                for c in chars.by_ref() {
                    if c == '\u{7}' || (previous == '\u{1b}' && c == '\\') {
                        break;
                    }
                    previous = c;
                }
            }
            // Any other two-byte escape is dropped whole.
            Some(_) | None => {}
        }
    }
    out
}

pub(crate) fn fold_blank(lines: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut run = 0usize;
    for line in lines {
        if line.trim().is_empty() {
            run = run.saturating_add(1);
            if run < BLANK_RUN_FOLDS_AT {
                out.push(line.clone());
            } else if run == BLANK_RUN_FOLDS_AT {
                out.truncate(
                    out.len()
                        .saturating_sub(BLANK_RUN_FOLDS_AT.saturating_sub(1)),
                );
                out.push(String::new());
            }
        } else {
            run = 0;
            out.push(line.clone());
        }
    }
    out
}

/// A line with its numbers, hashes and paths replaced by placeholders:
/// what `Compiling a v0.1.0` and `Compiling b v0.2.3` have in common.
fn template_of(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for token in line.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        if token.contains('/') || token.contains('\\') {
            out.push_str("<path>");
        } else if token.len() >= 7 && token.chars().all(|c| c.is_ascii_hexdigit()) {
            out.push_str("<hash>");
        } else {
            let mut in_digits = false;
            for c in token.chars() {
                if c.is_ascii_digit() {
                    if !in_digits {
                        out.push('#');
                    }
                    in_digits = true;
                } else {
                    in_digits = false;
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Lines sharing a template, when there are `TEMPLATE_FOLDS_AT` or
/// more of them, are said once with a count — and only when the fold
/// saves more bytes than the count costs, so this pass cannot grow a
/// text even locally. An `exempt` line (one a filter keeps, or one
/// inside a kept diagnostic) is never folded: its neighbours are its
/// meaning.
pub(crate) fn dedup_templates(lines: &[String], exempt: &[bool]) -> Vec<String> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, line) in lines.iter().enumerate() {
        let kept = exempt.get(index).copied().unwrap_or(false);
        if kept || line.trim().is_empty() || is_protected(line) {
            continue;
        }
        groups.entry(template_of(line)).or_default().push(index);
    }
    let mut drop = vec![false; lines.len()];
    let mut suffix: BTreeMap<usize, String> = BTreeMap::new();
    for members in groups.values() {
        if members.len() < TEMPLATE_FOLDS_AT {
            continue;
        }
        let Some((&first, rest)) = members.split_first() else {
            continue;
        };
        let note = format!(" [×{} similar lines]", members.len());
        let saved: usize = rest
            .iter()
            .filter_map(|&i| lines.get(i))
            .map(|l| l.len().saturating_add(1))
            .sum();
        if saved <= note.len() {
            continue;
        }
        for &i in rest {
            if let Some(slot) = drop.get_mut(i) {
                *slot = true;
            }
        }
        suffix.insert(first, note);
    }
    lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop.get(*i).copied().unwrap_or(false))
        .map(|(i, line)| match suffix.get(&i) {
            Some(note) => format!("{line}{note}"),
            None => line.clone(),
        })
        .collect()
}

pub(crate) fn cut_long_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if line.len() <= LONG_LINE_BYTES || is_protected(line) {
                return line.clone();
            }
            let at = boundary_before(line, LONG_LINE_BYTES);
            let dropped = line.len().saturating_sub(at);
            format!(
                "{} [line truncated: {dropped} bytes]",
                line.get(..at).unwrap_or_default()
            )
        })
        .collect()
}

/// How many lines survive a truncation, and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cuts {
    pub(crate) max_lines: usize,
    pub(crate) head: usize,
    pub(crate) tail: usize,
    pub(crate) middle: usize,
}

/// Head, tail, and the middle by priority: the marked middle lines
/// sorted by (priority, position), the first `middle` of them kept in
/// position, and every gap said as a count.
pub(crate) fn truncate(lines: &[String], marks: &[Option<Priority>], cuts: Cuts) -> Vec<String> {
    if lines.len() <= cuts.max_lines {
        return lines.to_vec();
    }
    let tail_from = lines.len().saturating_sub(cuts.tail).max(cuts.head);
    let mut candidates: Vec<(Priority, usize)> = (cuts.head..tail_from)
        .filter_map(|i| marks.get(i).copied().flatten().map(|p| (p, i)))
        .collect();
    candidates.sort();
    let mut kept: Vec<usize> = candidates
        .iter()
        .take(cuts.middle)
        .map(|&(_, i)| i)
        .collect();
    kept.sort_unstable();
    let mut out: Vec<String> = lines.get(..cuts.head).unwrap_or_default().to_vec();
    let mut cursor = cuts.head;
    for i in kept {
        push_gap(&mut out, i.saturating_sub(cursor));
        if let Some(line) = lines.get(i) {
            out.push(line.clone());
        }
        cursor = i.saturating_add(1);
    }
    push_gap(&mut out, tail_from.saturating_sub(cursor));
    out.extend(lines.get(tail_from..).unwrap_or_default().iter().cloned());
    out
}

fn push_gap(out: &mut Vec<String>, omitted: usize) {
    if omitted > 0 {
        out.push(format!("[… {omitted} lines omitted …]"));
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    #[test]
    fn escapes_and_carriage_returns_leave_what_a_terminal_showed() {
        let out = strip_ansi(&lines(
            "\x1b[1m\x1b[32mok\x1b[0m\n0%\r50%\r100%\ndone\r\n\x1b]0;title\x07x",
        ));
        assert_eq!(out, vec!["ok", "100%", "done", "x"]);
    }

    #[test]
    fn three_blank_lines_fold_and_two_do_not() {
        assert_eq!(fold_blank(&lines("a\n\n\n\nb")), vec!["a", "", "b"]);
        assert_eq!(fold_blank(&lines("a\n\n\n\n\n\n\nb")), vec!["a", "", "b"]);
        assert_eq!(fold_blank(&lines("a\n\n\nb")), vec!["a", "", "", "b"]);
        assert_eq!(fold_blank(&lines("a\n\nb")), vec!["a", "", "b"]);
    }

    #[test]
    fn templates_fold_at_four_and_interleaving_does_not_matter() {
        let text = "Compiling dep1 v0.1.0\nx\nCompiling dep2 v0.2.3\nCompiling dep3 v1.0.0\ny\nCompiling dep4 v2.0.0\n";
        let none = vec![false; 6];
        let out = dedup_templates(&lines(text), &none);
        assert_eq!(
            out,
            vec!["Compiling dep1 v0.1.0 [×4 similar lines]", "x", "y"]
        );
        let three = "Compiling dep1 v0.1.0\nCompiling dep2 v0.2.3\nCompiling dep3 v1.0.0\n";
        assert_eq!(dedup_templates(&lines(three), &none).len(), 3);
        let mut exempt = none;
        exempt[2] = true;
        assert_eq!(
            dedup_templates(&lines(text), &exempt).len(),
            6,
            "three left are not four"
        );
        assert_eq!(template_of("  --> src/a.rs:1:2"), "--> <path>");
        assert_eq!(template_of("commit deadbeef01 x9"), "commit <hash> x#");
    }

    #[test]
    fn a_fold_that_would_not_save_bytes_is_not_made() {
        let out = dedup_templates(&lines("a\na\na\na\n"), &[false; 4]);
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn a_long_line_is_cut_on_a_character_boundary_and_a_url_line_is_not() {
        let long = "字".repeat(1000);
        let out = cut_long_lines(std::slice::from_ref(&long));
        assert!(out[0].len() < long.len());
        assert!(out[0].contains("[line truncated:"));
        let url = format!("https://x/{}", "a".repeat(3000));
        assert_eq!(cut_long_lines(std::slice::from_ref(&url))[0], url);
    }

    #[test]
    fn the_middle_keeps_the_highest_priorities_in_position() {
        let all: Vec<String> = (0..50).map(|n| format!("line {n}")).collect();
        let mut marks = vec![None; 50];
        marks[20] = Some(Priority::Warning);
        marks[30] = Some(Priority::Error);
        marks[35] = Some(Priority::Note);
        let cuts = Cuts {
            max_lines: 10,
            head: 2,
            tail: 2,
            middle: 2,
        };
        let out = truncate(&all, &marks, cuts);
        assert_eq!(
            out,
            vec![
                "line 0",
                "line 1",
                "[… 18 lines omitted …]",
                "line 20",
                "[… 9 lines omitted …]",
                "line 30",
                "[… 17 lines omitted …]",
                "line 48",
                "line 49"
            ]
        );
        assert_eq!(
            truncate(
                &all,
                &marks,
                Cuts {
                    max_lines: 50,
                    ..cuts
                }
            ),
            all
        );
    }
}
