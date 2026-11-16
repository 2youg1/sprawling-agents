// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::{Duration, Instant};

use crate::views::snapshot::start::TailFrom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Bind,

    OpenLedger,

    FoldTail { lines: u64, from: TailFrom },

    CutStanding,

    CutViews,

    Twin,

    StartWorker,
}

pub struct OpeningCost {
    clock: fn() -> Instant,
    began: Instant,
    last: Instant,
    laps: Vec<(Phase, Duration)>,
}

impl OpeningCost {
    pub fn begin(clock: fn() -> Instant) -> OpeningCost {
        let began = clock();
        OpeningCost {
            clock,
            began,
            last: began,
            laps: Vec::with_capacity(7),
        }
    }

    pub fn lap(&mut self, phase: Phase) {
        let now = (self.clock)();
        self.laps
            .push((phase, now.saturating_duration_since(self.last)));
        self.last = now;
    }

    pub fn began(&self) -> Instant {
        self.began
    }

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

pub fn millis(span: Duration) -> String {
    let micros = u64::try_from(span.as_micros()).unwrap_or(u64::MAX);
    format!(
        "{}.{:03}",
        micros.div_euclid(1_000),
        micros.rem_euclid(1_000)
    )
}
