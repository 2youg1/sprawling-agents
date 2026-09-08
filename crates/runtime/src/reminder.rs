// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How full the window is, from the provider's own count.
//!
//! Usage is the `input_tokens` the provider reported for the last call
//! — a fact — against the model's `context_tokens` from the endpoint
//! book. Never a byte count of the window, which is an estimate, and
//! never a tokenizer of this repository's own, which would be a second
//! one.
//!
//! Two thresholds, and each sounds exactly once per run. At a quarter
//! only the usage is reported; at two thirds the run is told that what
//! is left is still enough to write a handoff and `succeed`, and that
//! past this point it will not be. A gauge that jumps past both in one
//! reading sounds the higher one only: two sentences stacked at once
//! are noise, and the lower one has nothing left to say.

use kernel::Tokens;

/// Percent of the window at which usage is first reported.
const QUARTER: u64 = 25;
/// Percent of the window past which a handoff is due.
const TWO_THIRDS: u64 = 65;

/// Which thresholds have sounded. Monotone: a gauge never goes back to
/// a lower state, so a threshold cannot sound twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sounded {
    Nothing,
    Quarter,
    TwoThirds,
}

/// The line the run is given, with the figures it is computed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextReminder {
    /// A quarter of the window is used: the figures, nothing more.
    Usage { used: Tokens, window: Tokens },
    /// Two thirds are used: still enough to hand over, not for long.
    HandoverWindow { used: Tokens, window: Tokens },
}

impl ContextReminder {
    /// The sentence, defined here once. Changing it changes window bytes
    /// and passes through the SPEC.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            ContextReminder::Usage { used, window } => format!(
                "[context] {}% of the window used ({} of {} input tokens).",
                percent(*used, *window),
                used.get(),
                window.get()
            ),
            ContextReminder::HandoverWindow { used, window } => format!(
                "[context] {}% of the window used ({} of {} input tokens). What is left is \
                 still enough to write a handoff and `succeed`; past this point it will not be.",
                percent(*used, *window),
                used.get(),
                window.get()
            ),
        }
    }
}

/// Whole percent, floored; zero for a window nobody stated, full for a
/// product that will not fit.
fn percent(used: Tokens, window: Tokens) -> u64 {
    used.get()
        .checked_mul(100)
        .and_then(|scaled| scaled.checked_div(window.get()))
        .unwrap_or(if window.get() == 0 { 0 } else { u64::MAX })
}

/// One run's gauge. Fed the provider's `input_tokens` after every call;
/// answers a reminder the first time each threshold is crossed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextGauge {
    window: Tokens,
    sounded: Sounded,
}

impl ContextGauge {
    #[must_use]
    pub fn new(window: Tokens) -> ContextGauge {
        ContextGauge {
            window,
            sounded: Sounded::Nothing,
        }
    }

    /// Reads one usage figure. `None` when no threshold was newly
    /// crossed, and always `None` for a window of zero: without a
    /// denominator there is no percentage to report.
    pub fn observe(&mut self, used: Tokens) -> Option<ContextReminder> {
        if self.window.get() == 0 {
            return None;
        }
        let at = percent(used, self.window);
        let window = self.window;
        match self.sounded {
            Sounded::Nothing | Sounded::Quarter if at >= TWO_THIRDS => {
                self.sounded = Sounded::TwoThirds;
                Some(ContextReminder::HandoverWindow { used, window })
            }
            Sounded::Nothing if at >= QUARTER => {
                self.sounded = Sounded::Quarter;
                Some(ContextReminder::Usage { used, window })
            }
            Sounded::Nothing | Sounded::Quarter | Sounded::TwoThirds => None,
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

    fn gauge() -> ContextGauge {
        ContextGauge::new(Tokens::new(1_000))
    }

    #[test]
    fn each_threshold_sounds_once_and_only_on_the_way_up() {
        let mut gauge = gauge();
        assert_eq!(gauge.observe(Tokens::new(240)), None);
        assert_eq!(
            gauge.observe(Tokens::new(250)),
            Some(ContextReminder::Usage {
                used: Tokens::new(250),
                window: Tokens::new(1_000)
            })
        );
        assert_eq!(
            gauge.observe(Tokens::new(400)),
            None,
            "the quarter sounded already"
        );
        assert_eq!(
            gauge.observe(Tokens::new(650)),
            Some(ContextReminder::HandoverWindow {
                used: Tokens::new(650),
                window: Tokens::new(1_000)
            })
        );
        assert_eq!(
            gauge.observe(Tokens::new(900)),
            None,
            "and so did two thirds"
        );
    }

    #[test]
    fn a_jump_past_both_sounds_the_higher_one_and_spends_the_lower() {
        let mut gauge = gauge();
        assert!(matches!(
            gauge.observe(Tokens::new(700)),
            Some(ContextReminder::HandoverWindow { .. })
        ));
        assert_eq!(
            gauge.observe(Tokens::new(300)),
            None,
            "the quarter is spent too"
        );
    }

    #[test]
    fn no_window_means_no_percentage() {
        let mut gauge = ContextGauge::new(Tokens::new(0));
        assert_eq!(gauge.observe(Tokens::new(u64::MAX)), None);
    }

    #[test]
    fn the_sentences_carry_the_figures_the_provider_reported() {
        let usage = ContextReminder::Usage {
            used: Tokens::new(300),
            window: Tokens::new(1_000),
        };
        assert_eq!(
            usage.render(),
            "[context] 30% of the window used (300 of 1000 input tokens)."
        );
        let handover = ContextReminder::HandoverWindow {
            used: Tokens::new(700),
            window: Tokens::new(1_000),
        };
        assert!(
            handover
                .render()
                .starts_with("[context] 70% of the window used")
        );
        assert!(
            handover
                .render()
                .contains("still enough to write a handoff and `succeed`")
        );
    }

    #[test]
    fn an_overflowing_product_reads_as_full_rather_than_wrapping() {
        assert_eq!(percent(Tokens::new(u64::MAX), Tokens::new(1_000)), u64::MAX);
    }
}
