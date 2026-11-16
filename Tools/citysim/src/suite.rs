// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, Locator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Half {

    HeldIn,

    HeldOut,
}

impl Half {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Half::HeldIn => "held_in",
            Half::HeldOut => "held_out",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub at: Locator,
    pub half: Half,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub id: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub tried: u32,
    pub passed: u32,
}

impl Tally {
    #[must_use]
    pub fn per_mille(&self) -> u32 {
        if self.tried == 0 {
            return 0;
        }
        self.passed
            .saturating_mul(1000)
            .checked_div(self.tried)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub held_in: Tally,
    pub held_out: Tally,

    pub unknown: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    tasks: BTreeMap<String, Task>,
}

impl Suite {

    pub fn new(tasks: Vec<Task>) -> Result<Suite, AxError> {
        let mut held: BTreeMap<String, Task> = BTreeMap::new();
        for task in tasks {
            if task.id.trim().is_empty() {
                return Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "build a task suite",
                    "a task with no id".to_owned(),
                )
                .with_recovery("give every task an id; results are compared by it"));
            }
            if let Some(first) = held.get(&task.id) {
                return Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "build a task suite",
                    format!(
                        "{} is in {} and again in {}",
                        task.id,
                        first.half.as_str(),
                        task.half.as_str()
                    ),
                )
                .with_recovery(
                    "a task belongs to one half; the held-out set is worth nothing once the \
                     change has seen it",
                ));
            }
            held.insert(task.id.clone(), task);
        }
        Ok(Suite { tasks: held })
    }

    #[must_use]
    pub fn half(&self, half: Half) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|task| task.half == half)
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    #[must_use]
    pub fn report(&self, outcomes: &[Outcome]) -> Report {
        let mut held_in = Tally {
            tried: 0,
            passed: 0,
        };
        let mut held_out = Tally {
            tried: 0,
            passed: 0,
        };
        let mut unknown: u32 = 0;
        for outcome in outcomes {
            let Some(task) = self.tasks.get(&outcome.id) else {
                unknown = unknown.saturating_add(1);
                continue;
            };
            let tally = match task.half {
                Half::HeldIn => &mut held_in,
                Half::HeldOut => &mut held_out,
            };
            tally.tried = tally.tried.saturating_add(1);
            if outcome.passed {
                tally.passed = tally.passed.saturating_add(1);
            }
        }
        Report {
            held_in,
            held_out,
            unknown,
        }
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

    fn task(id: &str, half: Half) -> Task {
        Task {
            id: id.to_owned(),
            at: Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap(),
            half,
        }
    }

    #[test]
    fn a_task_in_both_halves_is_refused_where_it_is_written() {
        let leaked = Suite::new(vec![
            task("t1", Half::HeldIn),
            task("t2", Half::HeldOut),
            task("t1", Half::HeldOut),
        ]);
        let err = leaked.unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(err.recovery().contains("held-out"));
    }

    #[test]
    fn the_same_suite_runs_the_same_way_twice() {
        let suite = Suite::new(vec![
            task("b", Half::HeldIn),
            task("a", Half::HeldIn),
            task("c", Half::HeldOut),
        ])
        .unwrap();
        let first: Vec<&str> = suite
            .half(Half::HeldIn)
            .iter()
            .map(|t| t.id.as_str())
            .collect();
        let again: Vec<&str> = suite
            .half(Half::HeldIn)
            .iter()
            .map(|t| t.id.as_str())
            .collect();
        assert_eq!(first, vec!["a", "b"], "id order, not insertion order");
        assert_eq!(first, again);
    }

    #[test]
    fn a_report_keeps_the_halves_apart_and_says_what_it_did_not_recognise() {
        let suite = Suite::new(vec![
            task("a", Half::HeldIn),
            task("b", Half::HeldIn),
            task("c", Half::HeldOut),
        ])
        .unwrap();
        let report = suite.report(&[
            Outcome {
                id: "a".to_owned(),
                passed: true,
            },
            Outcome {
                id: "b".to_owned(),
                passed: false,
            },
            Outcome {
                id: "c".to_owned(),
                passed: true,
            },
            Outcome {
                id: "invented".to_owned(),
                passed: true,
            },
        ]);
        assert_eq!(report.held_in.tried, 2);
        assert_eq!(report.held_in.passed, 1);
        assert_eq!(report.held_in.per_mille(), 500);
        assert_eq!(report.held_out.per_mille(), 1000);
        assert_eq!(
            report.unknown, 1,
            "an answer to a question nobody asked is reported, not folded in"
        );
    }

    #[test]
    fn nothing_tried_is_zero_rather_than_a_division() {
        let empty = Tally {
            tried: 0,
            passed: 0,
        };
        assert_eq!(empty.per_mille(), 0);
    }
}
