// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The priority a core thread stands at: one step above normal, so the
//! commands it dispatches one step below never outrank the accounting
//! and the views, and back to normal once the thread has kept a core
//! busy through a whole window (sprawling-SPEC.md 8-93).

use std::time::{Duration, Instant};

/// How long a raised thread may stay busy before the valve lowers it.
pub(crate) const BUSY_LIMIT: Duration = Duration::from_secs(10);

/// Whether the core's threads are raised: the setting a person turns off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CorePriority {
    Raised,
    Normal,
}

/// Where a thread actually stands after asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Standing {
    Raised,
    Normal(Held),
}

/// Why a core thread stands at normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Held {
    ByTheSetting,
    /// The platform refused the raise; on Unix it needs `CAP_SYS_NICE`.
    Refused(String),
    ByTheValve,
}

/// Whether a raised thread keeps its level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Keep,
    Lower,
}

/// Raises the calling thread one step above normal, unless the setting
/// holds it at normal or the platform refuses.
pub(crate) fn raise_this_thread(setting: CorePriority) -> Standing {
    match setting {
        CorePriority::Raised => Standing::Raised,
        CorePriority::Normal => Standing::Normal(Held::ByTheSetting),
    }
}

/// Puts the calling thread back at normal.
pub(crate) fn lower_this_thread() -> Standing {
    Standing::Normal(Held::ByTheValve)
}

/// How busy a raised thread has been through the current window.
///
/// The time is a parameter: the valve reads no clock, so its judgement
/// is checked point by point on invented instants.
#[derive(Debug)]
pub(crate) struct Valve {
    limit: Duration,
    opened: Instant,
    busy: Duration,
    verdict: Verdict,
}

impl Valve {
    /// A valve whose first window opens at `now`.
    pub(crate) fn new(limit: Duration, now: Instant) -> Self {
        Self {
            limit,
            opened: now,
            busy: Duration::ZERO,
            verdict: Verdict::Keep,
        }
    }

    /// Records one turn: the thread woke at `woke` and blocked again at
    /// `slept`.
    pub(crate) fn record(&mut self, woke: Instant, slept: Instant) {
        drop((woke, slept, self.limit, self.opened, self.busy));
    }

    /// Whether the thread keeps its level; once `Lower`, always `Lower`.
    pub(crate) fn verdict(&self) -> Verdict {
        self.verdict
    }
}

#[cfg(test)]
mod tests;
