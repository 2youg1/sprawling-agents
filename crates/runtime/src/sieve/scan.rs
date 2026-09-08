// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which lines matter, and how much. Four hand-written linear scans
//! and no pattern engine: a regular expression here would put
//! backtracking between a model and its next turn, and the shapes
//! being looked for are all fixed-width or delimiter-bounded.
//!
//! Also the spans no stage may rewrite. A URL, a device code, a
//! `secret:` reference, a `path:line:col` and an exit code are the
//! things a reader acts on verbatim; a line carrying one is exempt
//! from folding and cutting, and counts as worth keeping.

/// How much a line matters. Declaration order is the order of
/// keeping: `Error` first, `Protected` last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Priority {
    Error,
    Failure,
    Assertion,
    Warning,
    Note,
    Protected,
}

const ERROR_WORDS: [&str; 1] = ["error"];
const FAILURE_WORDS: [&str; 4] = ["panicked", "fatal", "failure", "failed"];
const ASSERTION_WORDS: [&str; 1] = ["assertion"];
const WARNING_WORDS: [&str; 1] = ["warning"];
const NOTE_WORDS: [&str; 2] = ["note", "help"];
const RESULT_PHRASES: [&str; 2] = ["test result:", "exit code"];

/// The priority of one line, if it is worth keeping at all.
pub(crate) fn priority(line: &str) -> Option<Priority> {
    let lower = line.to_ascii_lowercase();
    if has_word(&lower, &ERROR_WORDS) {
        return Some(Priority::Error);
    }
    if has_word(&lower, &FAILURE_WORDS) || RESULT_PHRASES.iter().any(|p| lower.contains(p)) {
        return Some(Priority::Failure);
    }
    if has_word(&lower, &ASSERTION_WORDS) {
        return Some(Priority::Assertion);
    }
    if has_word(&lower, &WARNING_WORDS) || has_diagnostic_code(line) {
        return Some(Priority::Warning);
    }
    if has_word(&lower, &NOTE_WORDS) {
        return Some(Priority::Note);
    }
    if is_protected(line) {
        return Some(Priority::Protected);
    }
    None
}

/// A line no stage may fold, cut, or collapse into "unchanged".
pub(crate) fn is_protected(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.contains("://")
        || has_secret_ref(line)
        || has_location(line)
        || has_device_code(line)
        || RESULT_PHRASES.iter().any(|p| lower.contains(p))
        || lower.contains("exit status")
        || lower.contains("exited with")
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Keyword as a whole word: the characters on both sides are not word
/// characters, so `errors` and `error_count` do not count.
fn has_word(lower: &str, words: &[&str]) -> bool {
    words.iter().any(|word| {
        let mut from = 0;
        while let Some(found) = lower.get(from..).and_then(|rest| rest.find(word)) {
            let start = from.saturating_add(found);
            let end = start.saturating_add(word.len());
            let before = lower.get(..start).and_then(|s| s.chars().next_back());
            let after = lower.get(end..).and_then(|s| s.chars().next());
            if !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char) {
                return true;
            }
            from = end;
        }
        false
    })
}

/// Two to four capitals then three to five digits, bounded by
/// non-word characters: `E0308`, `GH006`, `TS2345`.
fn has_diagnostic_code(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        let upper = chars.get(at..).map_or(0, |s| {
            s.iter().take_while(|c| c.is_ascii_uppercase()).count()
        });
        if (2..=4).contains(&upper) {
            let digits_from = at.saturating_add(upper);
            let digits = chars
                .get(digits_from..)
                .map_or(0, |s| s.iter().take_while(|c| c.is_ascii_digit()).count());
            let end = digits_from.saturating_add(digits);
            let before_ok = !at
                .checked_sub(1)
                .and_then(|i| chars.get(i))
                .is_some_and(|c| is_word_char(*c));
            let after_ok = !chars.get(end).is_some_and(|c| is_word_char(*c));
            if (3..=5).contains(&digits) && before_ok && after_ok {
                return true;
            }
            at = end.max(at.saturating_add(1));
        } else {
            at = at.saturating_add(1);
        }
    }
    false
}

/// `secret:realm/name`: the reference form, never the plaintext.
fn has_secret_ref(line: &str) -> bool {
    line.match_indices("secret:").any(|(at, _)| {
        line.get(at.saturating_add(7)..)
            .and_then(|rest| rest.split_whitespace().next())
            .is_some_and(|token| token.contains('/'))
    })
}

/// `path:line:col` — a token with a path character, then two runs of
/// digits separated by colons. `src/main.rs:12:5`, `C:\a\b.rs:3:4`.
fn has_location(line: &str) -> bool {
    line.split_whitespace().any(|token| {
        let mut parts = token.rsplitn(3, ':');
        let col = parts.next().is_some_and(all_digits);
        let row = parts.next().is_some_and(all_digits);
        let path = parts.next().is_some_and(|p| {
            !p.is_empty() && (p.contains('.') || p.contains('/') || p.contains('\\'))
        });
        col && row && path
    })
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// A device code: groups of four capitals or digits joined by hyphens,
/// at least two groups, the whole token and nothing else.
fn has_device_code(line: &str) -> bool {
    line.split_whitespace().any(|token| {
        let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let groups: Vec<&str> = token.split('-').collect();
        groups.len() >= 2
            && groups.iter().all(|g| {
                g.len() == 4
                    && g.chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            })
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn keywords_count_as_whole_words_only() {
        assert_eq!(priority("error: aborting"), Some(Priority::Error));
        assert_eq!(priority("ERROR something"), Some(Priority::Error));
        assert_eq!(priority("no errors here"), None);
        assert_eq!(priority("error_count = 3"), None);
        assert_eq!(
            priority("thread 'main' panicked at"),
            Some(Priority::Failure)
        );
        assert_eq!(
            priority("test result: ok. 3 passed"),
            Some(Priority::Failure)
        );
        assert_eq!(
            priority("assertion `left == right` failed"),
            Some(Priority::Failure)
        );
        assert_eq!(priority("warning: unused"), Some(Priority::Warning));
        assert_eq!(priority("  = note: see"), Some(Priority::Note));
        assert_eq!(priority("   Compiling foo v0.1.0"), None);
    }

    #[test]
    fn the_four_shapes_are_found_without_a_pattern_engine() {
        assert!(has_diagnostic_code("code TS2345 here"));
        assert!(has_diagnostic_code("GH006"));
        // One capital is outside the SPEC's 2-4; `error[E0308]` is an
        // error by its keyword, not by its code.
        assert!(!has_diagnostic_code("E0308"));
        assert!(!has_diagnostic_code("compiled 437 files"));
        assert!(!has_diagnostic_code("ABCDEF123"));
        assert!(!has_diagnostic_code("xE0308"));
        assert!(has_location(" --> src/main.rs:12:5"));
        assert!(has_location("C:\\a\\b.rs:3:4"));
        assert!(!has_location("12:00:01 INFO"));
        assert!(has_device_code("enter ABCD-EFGH to continue"));
        assert!(!has_device_code("abcd-efgh"));
        assert!(has_secret_ref("key secret:openai/main"));
        assert!(!has_secret_ref("secret: yes"));
    }

    #[test]
    fn protected_lines_are_worth_keeping_at_the_lowest_tier() {
        assert_eq!(
            priority("see https://doc.rust-lang.org/x"),
            Some(Priority::Protected)
        );
        assert!(is_protected("process exited with code 1"));
        assert!(Priority::Error < Priority::Protected);
    }
}
