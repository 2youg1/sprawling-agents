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
//! a table with the reference class (`tools/citysim/spec/Bench.lean` §8-6). It
//! carries the digest of the bytes it was taken over the same way, so a
//! reading of a changed fixture cannot share a table with the register's
//! (section 3-8).

use std::time::Duration;

use kernel::{AxCode, AxError, B3Hash};
use sprawling::monitor::spread::{Share, Spread};

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

/// The conditions a reading was taken under: the machine class, and the
/// digest of the fixture whose bytes it measured. They always travel
/// together, so they are one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Taken {
    pub(crate) machine: MachineClass,
    pub(crate) fixture: B3Hash,
}

/// The heavy-load classes this bench Main measures, one scenario each.
/// The fifth class, multi-run parallel, is `sprawling`'s relay
/// instrument (citysim D18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Load {
    LargeLedgerFold,
    LargeWorktreePlacement,
    KeptWorktreeReclaim,
    LongSessionForwarding,
}

impl Load {
    /// Every variant, so a caller that walks the roster cannot leave one out.
    pub(crate) const ALL: [Load; 4] = [
        Load::LargeLedgerFold,
        Load::LargeWorktreePlacement,
        Load::KeptWorktreeReclaim,
        Load::LongSessionForwarding,
    ];

    /// The one spelling of each class, shared by every renderer.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Load::LargeLedgerFold => "large_ledger_fold",
            Load::LargeWorktreePlacement => "large_worktree_placement",
            Load::KeptWorktreeReclaim => "kept_worktree_reclaim",
            Load::LongSessionForwarding => "long_session_forwarding",
        }
    }
}

/// Which part of a scenario's path a reading prices.
///
/// The split is what keeps a disk floor from masking harness overhead and
/// keeps harness speed from standing in for the disk (`tools/citysim/spec/Bench.lean`
/// §8-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubMetric {
    /// The harness's own processing, with no durability commit under it.
    /// The two latency tiers apply to this.
    Harness,
    /// A path whose work is the disk work and whose seam does not split
    /// the two: priced whole.
    Whole,
}

impl SubMetric {
    /// The one spelling of each sub-metric, shared by every renderer.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            SubMetric::Harness => "harness",
            SubMetric::Whole => "whole",
        }
    }
}

/// One measurement: the latency distribution of one scenario at one
/// sub-metric, under the conditions it was taken in.
#[derive(Debug)]
pub(crate) struct Reading {
    load: Load,
    sub: SubMetric,
    taken: Taken,
    spread: Spread,
}

impl Reading {
    /// The one construction point: the shares are read by the product's
    /// one percentile reading, `Spread`, nearest rank
    /// (`crates/sprawling/Spec.lean` §8-129-2).
    ///
    /// # Errors
    /// Refuses a scenario that sampled nothing, because a reading with no
    /// distribution behind it would be a number nobody can reproduce.
    pub(crate) fn of(
        load: Load,
        sub: SubMetric,
        taken: Taken,
        samples: Vec<Duration>,
    ) -> Result<Reading, String> {
        let mut samples = samples.into_iter();
        let head = samples.next().ok_or_else(sampled_nothing)?;
        Ok(Reading {
            load,
            sub,
            taken,
            spread: Spread::of(head, samples),
        })
    }

    /// The reading line: fixed key order, integer microseconds.
    pub(crate) fn line(&self) -> String {
        format!(
            "perf load={} sub={} machine_class={} fixture={} samples={} floor_us={} p50_us={} p95_us={} p99_us={}",
            self.load.as_str(),
            self.sub.as_str(),
            self.taken.machine.as_str(),
            citysim::fixture_label(&self.taken.fixture),
            self.spread.samples(),
            self.spread.floor().as_micros(),
            self.spread.p(Share::P50).as_micros(),
            self.spread.p(Share::P95).as_micros(),
            self.spread.p(Share::P99).as_micros()
        )
    }
}

/// The failure a scenario with no samples deserves: one error shape,
/// three mandatory parts, no new code (`tools/citysim/spec/Bench.lean` §8-6).
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
