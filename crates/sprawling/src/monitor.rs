// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The performance monitor's history: whether anybody is watching, and
//! the last [`CAPACITY`] samples they read (sprawling-SPEC.md 8-90).
//!
//! Nobody watching costs nothing: no counter is read and the history
//! holds no memory. Where the counters come from is the caller's
//! reading function; this module touches no platform interface.

pub mod top;

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Samples kept: one a second for five minutes.
pub const CAPACITY: usize = 300;

/// The most the server-side history may occupy.
pub const HISTORY_BUDGET: usize = 64 * 1024;

const _: () = assert!(
    match CAPACITY.checked_mul(size_of::<Sample>()) {
        Some(bytes) => bytes <= HISTORY_BUDGET,
        None => false,
    },
    "the monitor history outgrew HISTORY_BUDGET"
);

/// One reading of every counter the monitor shows, in integers because
/// it travels on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sample {
    pub core_cpu_permille: u64,
    pub core_private_bytes: u64,
    pub core_working_set_bytes: u64,
    pub core_read_bytes: u64,
    pub core_written_bytes: u64,
    pub machine_cpu_permille: u64,
    pub machine_available_bytes: u64,
    pub volume_free_bytes: u64,
    pub ledger_queue_depth: u64,
    pub durable_lag: u64,
    pub relay_p50_nanos: u64,
    pub event_to_screen_p50_nanos: u64,
    pub queued_runs: u64,
}

/// The watcher count and the history it gates.
#[derive(Debug, Default)]
pub struct Monitor {
    watchers: Arc<AtomicUsize>,
    history: VecDeque<Sample>,
}

/// One person or agent watching. Dropping it stops them counting.
#[derive(Debug)]
pub struct Watch {
    watchers: Arc<AtomicUsize>,
}

impl Monitor {
    /// A monitor nobody watches yet; it allocates no history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one more watcher until the returned [`Watch`] is dropped.
    #[must_use]
    pub fn watch(&self) -> Watch {
        self.watchers.fetch_add(1, Ordering::Relaxed);
        Watch {
            watchers: Arc::clone(&self.watchers),
        }
    }

    /// Called once a second. While somebody watches, calls `read` once
    /// and keeps its sample, dropping the oldest beyond [`CAPACITY`];
    /// while nobody does, calls nothing and releases the history.
    pub fn tick(&mut self, read: impl FnOnce() -> Sample) {
        if self.watchers.load(Ordering::Relaxed) == 0 {
            self.history = VecDeque::new();
            return;
        }
        if self.history.len() == CAPACITY {
            self.history.pop_front();
        }
        self.history
            .reserve_exact(CAPACITY.saturating_sub(self.history.len()));
        self.history.push_back(read());
    }

    /// The kept samples, oldest first.
    pub fn history(&self) -> impl Iterator<Item = &Sample> {
        self.history.iter()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.watchers.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[path = "monitor/tests.rs"]
mod tests;
