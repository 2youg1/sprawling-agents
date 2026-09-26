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

/// `thread-priority`'s cross-platform scale runs 0 to 99; 70 is
/// `THREAD_PRIORITY_ABOVE_NORMAL` on Windows and 50 is normal.
const ONE_STEP_ABOVE_NORMAL: u8 = 70;
const NORMAL: u8 = 50;

/// A core thread's standing and the valve that watches it, carried
/// through the thread's loop.
#[derive(Debug)]
pub(crate) struct CoreThread {
    name: &'static str,
    standing: Standing,
    valve: Valve,
}

impl CoreThread {
    /// Raises the calling thread as the setting allows, and opens its
    /// valve at `now`. A refusal is told to the person once, here.
    pub(crate) fn raise(name: &'static str, setting: CorePriority, now: Instant) -> Self {
        let standing = raise_this_thread(setting);
        if let Standing::Normal(Held::Refused(reason)) = &standing {
            eprintln!("thread {name} stays at normal priority: {reason}");
        }
        Self {
            name,
            standing,
            valve: Valve::new(BUSY_LIMIT, now),
        }
    }

    /// Records one turn and, the first time the valve says `Lower` while
    /// the thread stands raised, lowers the thread and tells the person.
    pub(crate) fn record_turn_lowering_when_busy(&mut self, woke: Instant, slept: Instant) {
        self.valve.record(woke, slept);
        match (&self.standing, self.valve.verdict()) {
            (Standing::Raised, Verdict::Lower) => {
                self.standing = match lower_this_thread() {
                    Ok(standing) => {
                        eprintln!(
                            "thread {} kept a core busy for {} s and is back at normal priority",
                            self.name,
                            BUSY_LIMIT.as_secs()
                        );
                        standing
                    }
                    Err(err) => {
                        eprintln!(
                            "thread {} kept a core busy for {} s and could not be lowered: {err}",
                            self.name,
                            BUSY_LIMIT.as_secs()
                        );
                        Standing::Raised
                    }
                };
            }
            (Standing::Raised | Standing::Normal(_), Verdict::Keep)
            | (Standing::Normal(_), Verdict::Lower) => {}
        }
    }
}

/// The person's setting, or `Normal` with the refusal told on stderr
/// when it cannot be read: raising has a machine-wide cost, so it waits
/// for a reading that allows it.
pub(crate) fn setting_telling_a_refusal() -> CorePriority {
    crate::person::core_priority().unwrap_or_else(|err| {
        eprintln!("the core stays at normal priority: {err}");
        CorePriority::Normal
    })
}

/// The async runtime the socket is served on, its workers raised as
/// `setting` allows, each with its valve.
///
/// # Errors
///
/// The runtime's own failure to start.
pub(crate) fn serving_runtime(setting: CorePriority) -> std::io::Result<tokio::runtime::Runtime> {
    drop(setting);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
}

/// Raises the calling thread one step above normal, unless the setting
/// holds it at normal or the platform refuses.
pub(crate) fn raise_this_thread(setting: CorePriority) -> Standing {
    match setting {
        CorePriority::Normal => Standing::Normal(Held::ByTheSetting),
        CorePriority::Raised => match set_this_thread(ONE_STEP_ABOVE_NORMAL) {
            Ok(()) => Standing::Raised,
            Err(err) => Standing::Normal(Held::Refused(err.to_string())),
        },
    }
}

/// Puts the calling thread back at normal.
///
/// # Errors
///
/// The platform's refusal; neither platform refuses a thread lowering
/// itself, so this reaches the person only as a defect report.
pub(crate) fn lower_this_thread() -> Result<Standing, thread_priority::Error> {
    set_this_thread(NORMAL).map(|()| Standing::Normal(Held::ByTheValve))
}

fn set_this_thread(level: u8) -> Result<(), thread_priority::Error> {
    let value = thread_priority::ThreadPriorityValue::try_from(level)
        .map_err(|_| thread_priority::Error::Priority("a level outside 0 to 99"))?;
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Crossplatform(
        value,
    ))
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
    /// A window closes at the first turn that ends past `limit` after it
    /// opened; the thread is lowered when it was busy for at least 15/16
    /// of that window, which leaves room for the blocking waits a busy
    /// but healthy thread still makes.
    pub(crate) fn record(&mut self, woke: Instant, slept: Instant) {
        self.busy = self
            .busy
            .saturating_add(slept.saturating_duration_since(woke));
        let window = slept.saturating_duration_since(self.opened);
        if window < self.limit {
            return;
        }
        if self.busy.saturating_mul(16) >= window.saturating_mul(15) {
            self.verdict = Verdict::Lower;
        }
        self.opened = slept;
        self.busy = Duration::ZERO;
    }

    /// Whether the thread keeps its level; once `Lower`, always `Lower`.
    pub(crate) fn verdict(&self) -> Verdict {
        self.verdict
    }
}

#[cfg(test)]
mod tests;
