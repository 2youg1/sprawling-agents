// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each phase of opening a served city cost, and the one line that
//! says it (sprawling-SPEC.md 8-121).
//!
//! Shape: value. It reads no clock of its own: the clock is the function
//! [`OpeningCost::begin`] is handed, which `listen` takes from the one
//! monotonic sampling point, `serving::standing::monotonic_now`
//! (sprawling-SPEC.md 8-93). A phase is the difference of two monotonic
//! samples, so a wall clock set back by a person cannot make one negative.

use std::time::{Duration, Instant};

/// One phase of opening a served city, in the order `listen` does them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The port taken.
    Bind,
    /// The writer lock taken, the format probed, the tail recovered.
    OpenLedger,
    /// Every line verified from genesis while the views and the standing
    /// fold it and the index is built.
    VerifyAndFold { lines: usize },
    /// The standing snapshot cut at the last line folded.
    CutStanding,
    /// The views snapshot cut at the same line.
    CutViews,
    /// The second copy of the views, made through the snapshot encoding
    /// (sprawling-SPEC.md 8-99).
    Twin,
    /// The writer thread started.
    StartWorker,
}

/// The phases of one opening as they are lapped, and how much of the
/// verifying pass the folds took.
pub(crate) struct OpeningCost {
    clock: fn() -> Instant,
    began: Instant,
    last: Instant,
    laps: Vec<(Phase, Duration)>,
    folded: Duration,
}

impl OpeningCost {
    /// Starts timing now, on `clock`.
    pub(crate) fn begin(clock: fn() -> Instant) -> OpeningCost {
        let began = clock();
        OpeningCost {
            clock,
            began,
            last: began,
            laps: Vec::with_capacity(7),
            folded: Duration::ZERO,
        }
    }

    /// Records `phase` as the span from the previous lap, or from the
    /// beginning, to now.
    pub(crate) fn lap(&mut self, phase: Phase) {
        let now = (self.clock)();
        self.laps
            .push((phase, now.saturating_duration_since(self.last)));
        self.last = now;
    }

    /// Runs `work` and counts how long it took as folding, the part of
    /// the verifying pass the line names separately.
    pub(crate) fn folding<R>(&mut self, work: impl FnOnce() -> R) -> R {
        let from = (self.clock)();
        let done = work();
        self.folded = self
            .folded
            .saturating_add((self.clock)().saturating_duration_since(from));
        done
    }

    /// The one rendering: the whole opening, then each phase in the order
    /// it was lapped.
    pub(crate) fn line(&self) -> String {
        let phases: Vec<String> = self
            .laps
            .iter()
            .map(|&(phase, took)| self.phase_part(phase, took))
            .collect();
        format!(
            "opened the city in {} ms: {}",
            millis(self.last.saturating_duration_since(self.began)),
            phases.join(", ")
        )
    }

    fn phase_part(&self, phase: Phase, took: Duration) -> String {
        match phase {
            Phase::Bind => format!("bind {} ms", millis(took)),
            Phase::OpenLedger => format!("open the ledger {} ms", millis(took)),
            Phase::VerifyAndFold { lines } => format!(
                "verify and fold {lines} lines {} ms (folding {} ms of it)",
                millis(took),
                millis(self.folded)
            ),
            Phase::CutStanding => format!("cut the standing snapshot {} ms", millis(took)),
            Phase::CutViews => format!("cut the views snapshot {} ms", millis(took)),
            Phase::Twin => format!("copy the views {} ms", millis(took)),
            Phase::StartWorker => format!("start the worker {} ms", millis(took)),
        }
    }
}

/// A span in milliseconds with three decimals, converted from whole
/// microseconds rather than through a float. A span past `u64::MAX`
/// microseconds, half a million years, is written as that maximum.
pub(crate) fn millis(span: Duration) -> String {
    let micros = u64::try_from(span.as_micros()).unwrap_or(u64::MAX);
    format!(
        "{}.{:03}",
        micros.div_euclid(1_000),
        micros.rem_euclid(1_000)
    )
}
