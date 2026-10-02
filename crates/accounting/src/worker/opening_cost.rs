// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each phase of opening a served city cost, and the one line that
//! says it (`crates/sprawling/Spec.lean` §8-121).
//!
//! Shape: value. It reads no clock of its own: the clock is the function
//! [`OpeningCost::begin`] is handed, which `listen` takes from the one
//! monotonic sampling point, `serving::standing::monotonic_now`
//! (`crates/sprawling/Spec.lean` §8-93). A phase is the difference of two monotonic
//! samples, so a wall clock set back by a person cannot make one negative.

use std::time::{Duration, Instant};

use crate::views::snapshot::start::TailFrom;

/// One phase of opening a served city, in the order `listen` does them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The port taken.
    Bind,
    /// The writer lock taken, the format probed, the tail recovered.
    OpenLedger,
    /// The one pass that checks and folds the lines the views and the
    /// standing have not folded yet, from the earlier of their snapshots
    /// or from genesis (`crates/sprawling/Spec.lean` §8-122).
    FoldTail { lines: u64, from: TailFrom },
    /// The standing snapshot cut at the last line folded.
    CutStanding,
    /// The views snapshot cut at the same line.
    CutViews,
    /// The second copy of the views, made through the snapshot encoding
    /// (`crates/sprawling/Spec.lean` §8-99).
    Twin,
    /// The writer thread started.
    StartWorker,
}

/// The phases of one opening as they are lapped.
pub struct OpeningCost {
    clock: fn() -> Instant,
    began: Instant,
    last: Instant,
    laps: Vec<(Phase, Duration)>,
}

impl OpeningCost {
    /// Starts timing now, on `clock`.
    pub fn begin(clock: fn() -> Instant) -> OpeningCost {
        let began = clock();
        OpeningCost {
            clock,
            began,
            last: began,
            laps: Vec::with_capacity(7),
        }
    }

    /// Records `phase` as the span from the previous lap, or from the
    /// beginning, to now.
    pub fn lap(&mut self, phase: Phase) {
        let now = (self.clock)();
        self.laps
            .push((phase, now.saturating_duration_since(self.last)));
        self.last = now;
    }

    /// When opening began, the point every readiness moment is measured
    /// from, including the proof that ends after the first byte
    /// (`crates/sprawling/Spec.lean` §8-122).
    pub fn began(&self) -> Instant {
        self.began
    }

    /// The one rendering: the whole opening, then each phase in the order
    /// it was lapped.
    pub fn line(&self) -> String {
        let phases: Vec<String> = self
            .laps
            .iter()
            .map(|&(phase, took)| phase_part(phase, took))
            .collect();
        format!(
            "opened the city in {} ms: {}",
            millis(self.last.saturating_duration_since(self.began)),
            phases.join(", ")
        )
    }
}

fn phase_part(phase: Phase, took: Duration) -> String {
    match phase {
        Phase::Bind => format!("bind {} ms", millis(took)),
        Phase::OpenLedger => format!("open the ledger {} ms", millis(took)),
        Phase::FoldTail { lines, from } => {
            let from = match from {
                TailFrom::Snapshots => "the snapshots",
                TailFrom::Genesis => "genesis",
            };
            format!("fold {lines} lines from {from} {} ms", millis(took))
        }
        Phase::CutStanding => format!("cut the standing snapshot {} ms", millis(took)),
        Phase::CutViews => format!("cut the views snapshot {} ms", millis(took)),
        Phase::Twin => format!("copy the views {} ms", millis(took)),
        Phase::StartWorker => format!("start the worker {} ms", millis(took)),
    }
}

/// A span in milliseconds with three decimals, converted from whole
/// microseconds rather than through a float. A span past `u64::MAX`
/// microseconds, half a million years, is written as that maximum.
pub fn millis(span: Duration) -> String {
    let micros = u64::try_from(span.as_micros()).unwrap_or(u64::MAX);
    format!(
        "{}.{:03}",
        micros.div_euclid(1_000),
        micros.rem_euclid(1_000)
    )
}
