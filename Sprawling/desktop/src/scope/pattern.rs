// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug)]
pub(crate) struct Pattern {
    glob: Vec<char>,
}

impl Pattern {
    pub(crate) fn new(line: &str) -> Pattern {
        Pattern {
            glob: line.chars().collect(),
        }
    }

    pub(crate) fn matches(&self, named: &str) -> bool {
        let text: Vec<char> = named.chars().collect();
        matches(&self.glob, &text)
    }
}

fn matches(glob: &[char], text: &[char]) -> bool {
    let mut at_pattern: usize = 0;
    let mut at_text: usize = 0;

    let mut star: Option<(usize, usize)> = None;
    loop {
        let advanced = match (glob.get(at_pattern), text.get(at_text)) {
            (Some('*'), _) => {
                star = Some((at_pattern, at_text));
                at_pattern = at_pattern.saturating_add(1);
                true
            }
            (Some(expected), Some(actual)) if expected.eq_ignore_ascii_case(actual) => {
                at_pattern = at_pattern.saturating_add(1);
                at_text = at_text.saturating_add(1);
                true
            }
            (None, None) => return true,
            _mismatch => false,
        };
        if advanced {
            continue;
        }

        let Some((at_star, swallowed)) = star else {
            return false;
        };
        if text.get(swallowed).is_none() {
            return false;
        }
        let next = swallowed.saturating_add(1);
        star = Some((at_star, next));
        at_pattern = at_star.saturating_add(1);
        at_text = next;
    }
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
    fn a_star_stands_for_any_run_of_characters_including_none() {
        assert!(Pattern::new("*").matches(""));
        assert!(Pattern::new("*Notepad*").matches("a.txt — Notepad"));
        assert!(Pattern::new("*Notepad*").matches("Notepad"));
        assert!(Pattern::new("Note*pad").matches("Notepad"));
        assert!(Pattern::new("*.exe").matches("notepad.exe"));
        assert!(!Pattern::new("*.exe").matches("notepad.exe.bak"));
    }

    #[test]
    fn a_pattern_without_a_star_matches_the_whole_name_and_no_more() {
        assert!(Pattern::new("Calculator").matches("Calculator"));
        assert!(!Pattern::new("Calculator").matches("Calculator Plus"));
        assert!(!Pattern::new("Calculator").matches("The Calculator"));
        assert!(!Pattern::new("").matches("anything"));
    }

    #[test]
    fn matching_ignores_ascii_case() {
        assert!(Pattern::new("Notepad.exe").matches("NOTEPAD.EXE"));
        assert!(Pattern::new("*NOTEPAD*").matches("x — notepad"));
        assert!(!Pattern::new("notepad.exe").matches("notepad2.exe"));
    }

    #[test]
    fn a_name_that_nearly_matches_many_times_still_ends() {
        assert!(!Pattern::new("*aaaaaaaaab").matches("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(Pattern::new("*a*b*c").matches("zzazzbzzc"));
    }
}
