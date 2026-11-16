// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether a served city that just ended is left down, raised again, or
//! handed to the person (`crates/sprawling/spec/Supervising.lean` §8-109; the budget's
//! properties are proved in `crates/sprawling/spec/Supervising.lean`).
//!
//! The decision alone: no process, no clock. `children` runs the
//! processes, samples the time and reads a child's exit code as served
//! or failed; who closed the city and how is written in the child's own
//! handoff, which this module never needs to read.

mod children;

pub use children::{Child, Console, Ended, Window, supervise};

use kernel::{AxError, TimeMs};

/// How long a crash counts against the budget.
pub(crate) const CRASH_WINDOW_MS: u64 = 60_000;
/// How many crashes inside the window make the city degraded.
pub(crate) const CRASH_LIMIT: usize = 3;

/// The crashes still inside the window, oldest first; never more than
/// `CRASH_LIMIT`, because the one that reaches it ends the budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrashBudget {
    crashes: Vec<TimeMs>,
}

/// What the supervisor does after a served city ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Next {
    /// The person closed the city; supervising ends with it.
    Stop,
    /// A crash inside the budget: resume, then serve again.
    Restart(CrashBudget),
    /// The budget is spent: nothing restarts until the person lifts it.
    Degraded { crashes: usize, cause: String },
}

impl CrashBudget {
    /// A budget with no crash counted, which is also what lifting a
    /// degraded city starts from.
    pub(crate) fn fresh() -> Self {
        Self {
            crashes: Vec::with_capacity(CRASH_LIMIT),
        }
    }

    /// Spends this budget on one ending observed at `at`.
    pub(crate) fn after(mut self, served: &Result<(), AxError>, at: TimeMs) -> Next {
        match served {
            Ok(()) => Next::Stop,
            Err(failure) => {
                self.crashes
                    .retain(|crash| at.value().saturating_sub(crash.value()) < CRASH_WINDOW_MS);
                self.crashes.push(at);
                if self.crashes.len() >= CRASH_LIMIT {
                    Next::Degraded {
                        crashes: self.crashes.len(),
                        cause: failure.to_string(),
                    }
                } else {
                    Next::Restart(self)
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::{CrashBudget, Next};
    use kernel::{AxCode, AxError, TimeMs};

    fn crash() -> Result<(), AxError> {
        Err(AxError::failure(
            AxCode::StorageFatal,
            "serve a supervised city",
            "exit code: 101",
        )
        .with_recovery("start it again"))
    }

    /// Feeds crashes at the given seconds and returns every verdict.
    fn verdicts(seconds: &[u64]) -> Vec<Next> {
        let mut budget = CrashBudget::fresh();
        let mut seen = Vec::new();
        for s in seconds {
            let next = budget.clone().after(&crash(), TimeMs::new(s * 1_000));
            if let Next::Restart(kept) = &next {
                budget = kept.clone();
            }
            seen.push(next);
        }
        seen
    }

    fn restart(seconds: &[u64]) -> Next {
        let mut budget = CrashBudget::fresh();
        budget.crashes = seconds.iter().map(|s| TimeMs::new(s * 1_000)).collect();
        Next::Restart(budget)
    }

    #[test]
    fn three_crashes_inside_a_minute_degrade_and_older_ones_expire() {
        let degraded = Next::Degraded {
            crashes: 3,
            cause: crash().unwrap_err().to_string(),
        };
        assert_eq!(
            (
                verdicts(&[0, 10, 20]),
                verdicts(&[0, 10, 60]),
                CrashBudget::fresh().after(&Ok(()), TimeMs::new(5)),
            ),
            (
                vec![restart(&[0]), restart(&[0, 10]), degraded],
                vec![restart(&[0]), restart(&[0, 10]), restart(&[10, 60])],
                Next::Stop,
            )
        );
    }
}
