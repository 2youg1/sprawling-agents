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
//! Two thresholds, and each sounds exactly once per run. At the first
//! rung — 30% of the window — only the usage is reported; at the second
//! rung — 65% of the window
//! unless a layer moved it — the run is told that what
//! is left is still enough to write a handoff and `succeed`, and that
//! past this point it will not be. A gauge that jumps past both in one
//! reading sounds the higher one only: two sentences stacked at once
//! are noise, and the lower one has nothing left to say.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use kernel::Tokens;
use kernel::config::SecondThreshold;
use kernel::consts_policy::{CTX_REMINDER_FIRST_PERCENT, CTX_REMINDER_SECOND_DEFAULT};

/// Which thresholds have sounded. Monotone: a gauge never goes back to
/// a lower state, so a threshold cannot sound twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sounded {
    Nothing,
    First,
    Handover,
}

/// The line the run is given, with the figures it is computed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextReminder {
    /// The first rung is reached: the figures, nothing more.
    Usage { used: Tokens, window: Tokens },
    /// The second rung is reached: still enough to hand over, not for long.
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

/// The provider's count for the last call a run completed, shared with
/// whoever reports it.
///
/// One cell with one writer: the run records each count before its gauge
/// reads it, and `status` reads the cell when a model asks. A snapshot
/// frozen at dispatch cannot carry this figure, because nothing has been
/// called at dispatch and the figure moves every turn. Relaxed ordering
/// is enough: the value is one word and nothing is ordered against it.
#[derive(Debug, Clone, Default)]
pub struct ContextReading(Arc<AtomicU64>);

impl ContextReading {
    /// The last count recorded; zero before the first call completes.
    #[must_use]
    pub fn tokens(&self) -> Tokens {
        Tokens::new(self.0.load(Ordering::Relaxed))
    }

    pub(crate) fn record(&self, used: Tokens) {
        self.0.store(used.get(), Ordering::Relaxed);
    }
}

/// One run's gauge. Fed the provider's `input_tokens` after every call;
/// answers a reminder the first time each threshold is crossed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextGauge {
    window: Tokens,
    /// Where the second rung sits, as a whole percent: the city's rung
    /// when a layer stated one, else `CTX_REMINDER_SECOND_DEFAULT`.
    second_at: u64,
    sounded: Sounded,
}

impl ContextGauge {
    /// `second` is the configuration ladder's answer for this run, and
    /// `None` means no layer spoke: the default applies here, at the one
    /// place a rung is read.
    #[must_use]
    pub fn new(window: Tokens, second: Option<SecondThreshold>) -> ContextGauge {
        ContextGauge {
            window,
            second_at: second.map_or(CTX_REMINDER_SECOND_DEFAULT, SecondThreshold::percent),
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
            Sounded::Nothing | Sounded::First if at >= self.second_at => {
                self.sounded = Sounded::Handover;
                Some(ContextReminder::HandoverWindow { used, window })
            }
            Sounded::Nothing if at >= CTX_REMINDER_FIRST_PERCENT => {
                self.sounded = Sounded::First;
                Some(ContextReminder::Usage { used, window })
            }
            Sounded::Nothing | Sounded::First | Sounded::Handover => None,
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
        ContextGauge::new(Tokens::new(1_000), None)
    }

    #[test]
    fn each_threshold_sounds_once_and_only_on_the_way_up() {
        let mut gauge = gauge();
        assert_eq!(gauge.observe(Tokens::new(290)), None);
        assert_eq!(
            gauge.observe(Tokens::new(300)),
            Some(ContextReminder::Usage {
                used: Tokens::new(300),
                window: Tokens::new(1_000)
            })
        );
        assert_eq!(
            gauge.observe(Tokens::new(400)),
            None,
            "the first rung sounded already"
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
            "the first rung is spent too"
        );
    }

    /// A layer that moved the second rung moves the line with it: the
    /// handover claim is about the budget left at that rung, so the rung
    /// is the city's to set (`crates/kernel/Spec.lean` §8-22).
    #[test]
    fn the_second_rung_moves_when_a_layer_moved_it() {
        let moved = kernel::config::SecondThreshold::parse(31).unwrap();
        let mut gauge = ContextGauge::new(Tokens::new(1_000), Some(moved));
        assert_eq!(
            gauge.observe(Tokens::new(300)),
            Some(ContextReminder::Usage {
                used: Tokens::new(300),
                window: Tokens::new(1_000)
            })
        );
        assert_eq!(
            gauge.observe(Tokens::new(310)),
            Some(ContextReminder::HandoverWindow {
                used: Tokens::new(310),
                window: Tokens::new(1_000)
            }),
            "the moved rung sounds where it was moved to"
        );
    }

    #[test]
    fn no_window_means_no_percentage() {
        let mut gauge = ContextGauge::new(Tokens::new(0), None);
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
