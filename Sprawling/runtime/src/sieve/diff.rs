// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use kernel::Locator;

use crate::sieve::CommandKey;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SieveHistory(BTreeMap<CommandKey, Locator>);

impl SieveHistory {
    #[must_use]
    pub fn previous(&self, key: &CommandKey) -> Option<&Locator> {
        self.0.get(key)
    }

    pub fn record(&mut self, key: CommandKey, original: Locator) {
        self.0.insert(key, original);
    }
}

const UNCHANGED_RUN_FOLDS_AT: usize = 2;

fn unchanged_line(count: usize) -> String {
    format!("[unchanged: {count} lines, as in the previous run]")
}

pub(crate) fn only_changes(current: &[String], previous: &[String]) -> Vec<String> {
    let seen: BTreeSet<&str> = previous.iter().map(String::as_str).collect();
    let mut out = Vec::with_capacity(current.len());
    let mut run: Vec<&String> = Vec::new();
    let flush = |run: &mut Vec<&String>, out: &mut Vec<String>| {
        let bytes: usize = run.iter().map(|l| l.len().saturating_add(1)).sum();
        let count = run.len();
        if count >= UNCHANGED_RUN_FOLDS_AT && unchanged_line(count).len() < bytes {
            out.push(unchanged_line(count));
        } else {
            out.extend(run.iter().map(|l| (*l).clone()));
        }
        run.clear();
    };
    for line in current {
        if seen.contains(line.as_str()) {
            run.push(line);
        } else {
            flush(&mut run, &mut out);
            out.push(line.clone());
        }
    }
    flush(&mut run, &mut out);
    out
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

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    #[test]
    fn only_the_new_lines_are_spelled_out() {
        let previous = lines("a long enough line 1\na long enough line 2\na long enough line 3\nb");
        let current =
            lines("a long enough line 1\na long enough line 2\na long enough line 3\nNEW\nb");
        let out = only_changes(&current, &previous);
        assert_eq!(
            out,
            vec!["[unchanged: 3 lines, as in the previous run]", "NEW", "b"]
        );
    }

    #[test]
    fn a_short_run_is_left_as_it_was_and_a_url_is_never_folded() {
        let previous = lines("x\ny\nhttps://a.b/c\nhttps://d.e/f");
        let current = previous.clone();
        assert_eq!(only_changes(&current, &previous), current);
    }

    #[test]
    fn the_history_keeps_the_latest_per_key() {
        let key = CommandKey::of(&kernel::ExecArm::Shell {
            text: "ls".to_owned(),
        });
        let mut history = SieveHistory::default();
        assert!(history.previous(&key).is_none());
        let first = Locator::parse(&format!("cas:b3-{}", "0".repeat(64))).unwrap();
        let second = Locator::parse(&format!("cas:b3-{}", "1".repeat(64))).unwrap();
        history.record(key.clone(), first);
        history.record(key.clone(), second.clone());
        assert_eq!(history.previous(&key), Some(&second));
    }
}
