// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use kernel::{AxCode, AxError, B3Hash};
use sprawling::monitor::spread::{Share, Spread, micros};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MachineClass {
    General,
}

impl MachineClass {

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            MachineClass::General => "general",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Taken {
    pub(crate) machine: MachineClass,
    pub(crate) fixture: B3Hash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Load {
    LargeLedgerFold,
    LargeWorktreePlacement,
    KeptWorktreeReclaim,
    LongSessionForwarding,
}

impl Load {

    pub(crate) const ALL: [Load; 4] = [
        Load::LargeLedgerFold,
        Load::LargeWorktreePlacement,
        Load::KeptWorktreeReclaim,
        Load::LongSessionForwarding,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Load::LargeLedgerFold => "large_ledger_fold",
            Load::LargeWorktreePlacement => "large_worktree_placement",
            Load::KeptWorktreeReclaim => "kept_worktree_reclaim",
            Load::LongSessionForwarding => "long_session_forwarding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubMetric {

    Harness,

    Whole,
}

impl SubMetric {

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            SubMetric::Harness => "harness",
            SubMetric::Whole => "whole",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Reading {
    load: Load,
    sub: SubMetric,
    taken: Taken,
    spread: Spread,
}

impl Reading {

    pub(crate) fn of(
        load: Load,
        sub: SubMetric,
        taken: Taken,
        samples: Vec<Duration>,
    ) -> Result<Reading, String> {
        Ok(Reading {
            load,
            sub,
            taken,
            spread: spread_of(samples)?,
        })
    }

    pub(crate) fn line(&self) -> String {
        format!(
            "perf load={} sub={} machine_class={} fixture={} samples={} floor_us={} p50_us={} p95_us={} p99_us={}",
            self.load.as_str(),
            self.sub.as_str(),
            self.taken.machine.as_str(),
            citysim::fixture_label(&self.taken.fixture),
            self.spread.samples(),
            micros(self.spread.floor()),
            micros(self.spread.p(Share::P50)),
            micros(self.spread.p(Share::P95)),
            micros(self.spread.p(Share::P99))
        )
    }
}

pub(crate) fn spread_of(samples: Vec<Duration>) -> Result<Spread, String> {
    let mut samples = samples.into_iter();
    let head = samples.next().ok_or_else(sampled_nothing)?;
    Ok(Spread::of(head, samples))
}

pub(crate) fn row_line(row: &str, spread: &Spread, beside: &str) -> String {
    format!(
        "budget row={row} samples={} floor_us={} p50_us={} p95_us={} p99_us={} peak_us={}{beside}",
        spread.samples(),
        micros(spread.floor()),
        micros(spread.p(Share::P50)),
        micros(spread.p(Share::P95)),
        micros(spread.p(Share::P99)),
        micros(spread.peak())
    )
}

fn sampled_nothing() -> String {
    format!(
        "{}",
        AxError::failure(
            AxCode::InvalidArgs,
            "build a reading",
            "a scenario that sampled nothing"
        )
        .with_recovery("run the scenario with the registered load, which always samples")
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
