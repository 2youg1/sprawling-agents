// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One load-scenario measurement, spelled the same way every time.
//!
//! The line grammar is this module's alone: `Reading::line` is the only
//! renderer, and the tests compare it byte for byte. A reading carries
//! its machine class as a field rather than inheriting it from a header,
//! so a reading that arrived from another machine cannot silently share
//! a table with the reference class (citysim-SPEC.md section 8-6).

use std::time::Duration;

use kernel::{AxCode, AxError};

/// The class of machine a reading was taken on.
///
/// `General` is the reference class: disk, memory and CPU all at general
/// level. A reading from a class not spelled here does not enter the
/// register beside a reference reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MachineClass {
    General,
}

impl MachineClass {
    /// The one spelling of the class, shared by every renderer.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            MachineClass::General => "general",
        }
    }
}

/// The four heavy-load classes, one bench scenario each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Load {
    MultiRunParallel,
    LargeLedgerFold,
    LargeWorktreePlacement,
    LongSessionForwarding,
}

impl Load {
    /// Every variant, so a caller that walks the roster cannot leave one out.
    pub(crate) const ALL: [Load; 4] = [
        Load::MultiRunParallel,
        Load::LargeLedgerFold,
        Load::LargeWorktreePlacement,
        Load::LongSessionForwarding,
    ];

    /// The one spelling of each class, shared by every renderer.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Load::MultiRunParallel => "multi_run_parallel",
            Load::LargeLedgerFold => "large_ledger_fold",
            Load::LargeWorktreePlacement => "large_worktree_placement",
            Load::LongSessionForwarding => "long_session_forwarding",
        }
    }
}

/// Which part of a scenario's path a reading prices.
///
/// The split is what keeps a disk floor from masking harness overhead and
/// keeps harness speed from standing in for the disk (citysim-SPEC.md
/// section 8-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubMetric {
    /// The harness's own processing, with no durability commit under it.
    /// The two latency tiers apply to this.
    Harness,
    /// The same path with its durability commit under it. The disk's
    /// physical floor is the reference for it, not a latency tier.
    Persist,
    /// A path whose work is the disk work and whose seam does not split
    /// the two: priced whole.
    Whole,
}

impl SubMetric {
    /// The one spelling of each sub-metric, shared by every renderer.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            SubMetric::Harness => "harness",
            SubMetric::Persist => "persist",
            SubMetric::Whole => "whole",
        }
    }
}

/// One measurement: the latency distribution of one scenario at one
/// sub-metric, on one machine class.
#[derive(Debug)]
pub(crate) struct Reading {
    load: Load,
    sub: SubMetric,
    machine: MachineClass,
    samples: u64,
    p50: Duration,
    p95: Duration,
    p99: Duration,
}

impl Reading {
    /// The one construction point: percentiles off the sample vector.
    ///
    /// # Errors
    /// Refuses a scenario that sampled nothing, because a reading with no
    /// distribution behind it would be a number nobody can reproduce.
    pub(crate) fn of(
        load: Load,
        sub: SubMetric,
        machine: MachineClass,
        mut samples: Vec<Duration>,
    ) -> Result<Reading, String> {
        samples.sort();
        let count = samples.len();
        if count == 0 {
            return Err(sampled_nothing());
        }
        let at = |percent: usize| -> Result<Duration, String> {
            let index = count
                .saturating_mul(percent)
                .saturating_div(100)
                .min(count.saturating_sub(1));
            samples.get(index).copied().ok_or_else(sampled_nothing)
        };
        Ok(Reading {
            load,
            sub,
            machine,
            samples: u64::try_from(count).map_err(|why| format!("count the samples: {why}"))?,
            p50: at(50)?,
            p95: at(95)?,
            p99: at(99)?,
        })
    }

    /// The reading line: fixed key order, integer microseconds.
    pub(crate) fn line(&self) -> String {
        format!(
            "perf load={} sub={} machine_class={} samples={} p50_us={} p95_us={} p99_us={}",
            self.load.as_str(),
            self.sub.as_str(),
            self.machine.as_str(),
            self.samples,
            self.p50.as_micros(),
            self.p95.as_micros(),
            self.p99.as_micros()
        )
    }
}

/// The failure a scenario with no samples deserves: one error shape,
/// three mandatory parts, no new code (citysim-SPEC.md section 8-6).
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
