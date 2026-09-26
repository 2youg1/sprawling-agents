// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether a served city that just ended is left down, raised again, or
//! handed to the person (sprawling-SPEC.md section 8-109).
//!
//! The decision alone: no process, no clock. `children` runs the
//! processes and samples the time; how an ending reads is
//! `Closing::of`, so this module never classifies an exit itself.

mod children;

pub use children::{Child, Console, Ended, Window, supervise};

use crate::assembly::Closing;
use kernel::TimeMs;

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
    pub(crate) fn after(mut self, closing: &Closing, at: TimeMs) -> Next {
        match closing {
            Closing::Chosen => Next::Stop,
            Closing::Broken { cause } => {
                self.crashes
                    .retain(|crash| at.value().saturating_sub(crash.value()) < CRASH_WINDOW_MS);
                self.crashes.push(at);
                if self.crashes.len() >= CRASH_LIMIT {
                    Next::Degraded {
                        crashes: self.crashes.len(),
                        cause: cause.clone(),
                    }
                } else {
                    Next::Restart(self)
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{CrashBudget, Next};
    use crate::assembly::Closing;
    use kernel::TimeMs;

    fn crash() -> Closing {
        Closing::Broken {
            cause: "exit code: 101".to_owned(),
        }
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
            cause: "exit code: 101".to_owned(),
        };
        assert_eq!(
            (
                verdicts(&[0, 10, 20]),
                verdicts(&[0, 10, 60]),
                CrashBudget::fresh().after(&Closing::Chosen, TimeMs::new(5)),
            ),
            (
                vec![restart(&[0]), restart(&[0, 10]), degraded],
                vec![restart(&[0]), restart(&[0, 10]), restart(&[10, 60])],
                Next::Stop,
            )
        );
    }
}
