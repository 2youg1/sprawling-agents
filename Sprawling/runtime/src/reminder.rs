// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use kernel::Tokens;
use kernel::config::SecondThreshold;
use kernel::consts_policy::{CTX_REMINDER_FIRST_PERCENT, CTX_REMINDER_SECOND_DEFAULT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sounded {
    Nothing,
    First,
    Handover,
    Locked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextReminder {
    Usage { used: Tokens, window: Tokens },

    HandoverWindow { used: Tokens, window: Tokens },

    LockReached { used: Tokens, window: Tokens },
}

impl ContextReminder {
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
            ContextReminder::LockReached { used, window } => format!(
                "[context] the building's context lock is full ({} of {} input tokens). Write \
                 Handoff.md and call `succeed` now so the next run continues in this room under \
                 the same lock.",
                used.get(),
                window.get()
            ),
        }
    }

    #[must_use]
    pub fn asks_succession(&self) -> bool {
        matches!(self, ContextReminder::LockReached { .. })
    }
}

fn percent(used: Tokens, window: Tokens) -> u64 {
    used.get()
        .checked_mul(100)
        .and_then(|scaled| scaled.checked_div(window.get()))
        .unwrap_or(if window.get() == 0 { 0 } else { u64::MAX })
}

#[derive(Debug, Clone, Default)]
pub struct ContextReading(Arc<AtomicU64>);

impl ContextReading {
    #[must_use]
    pub fn tokens(&self) -> Tokens {
        Tokens::new(self.0.load(Ordering::Relaxed))
    }

    pub(crate) fn record(&self, used: Tokens) {
        self.0.store(used.get(), Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextGauge {
    window: Tokens,

    second_at: u64,

    locked: bool,
    sounded: Sounded,
}

impl ContextGauge {
    #[must_use]
    pub fn new(window: Tokens, second: Option<SecondThreshold>) -> ContextGauge {
        ContextGauge::locked(window, second, false)
    }

    #[must_use]
    pub fn locked(window: Tokens, second: Option<SecondThreshold>, locked: bool) -> ContextGauge {
        ContextGauge {
            window,
            second_at: second.map_or(CTX_REMINDER_SECOND_DEFAULT, SecondThreshold::percent),
            locked,
            sounded: Sounded::Nothing,
        }
    }

    pub fn observe(&mut self, used: Tokens) -> Option<ContextReminder> {
        if self.window.get() == 0 {
            return None;
        }
        let at = percent(used, self.window);
        let window = self.window;
        if self.locked && at >= 100 && self.sounded != Sounded::Locked {
            self.sounded = Sounded::Locked;
            return Some(ContextReminder::LockReached { used, window });
        }
        match self.sounded {
            Sounded::Nothing | Sounded::First if at >= self.second_at => {
                self.sounded = Sounded::Handover;
                Some(ContextReminder::HandoverWindow { used, window })
            }
            Sounded::Nothing if at >= CTX_REMINDER_FIRST_PERCENT => {
                self.sounded = Sounded::First;
                Some(ContextReminder::Usage { used, window })
            }
            Sounded::Nothing | Sounded::First | Sounded::Handover | Sounded::Locked => None,
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

    #[test]
    fn a_building_lock_sounds_when_the_effective_window_fills() {
        let mut gauge = ContextGauge::locked(Tokens::new(256), None, true);
        assert!(matches!(
            gauge.observe(Tokens::new(200)),
            Some(ContextReminder::HandoverWindow { .. })
        ));
        assert_eq!(
            gauge.observe(Tokens::new(256)),
            Some(ContextReminder::LockReached {
                used: Tokens::new(256),
                window: Tokens::new(256)
            })
        );
        assert!(gauge.observe(Tokens::new(256)).is_none());
    }
}
