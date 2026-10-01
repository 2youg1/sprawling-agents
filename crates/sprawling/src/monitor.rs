// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The performance monitor's history: whether anybody is watching, and
//! the last [`CAPACITY`] samples they read (sprawling-SPEC.md 8-94).
//!
//! Nobody watching costs nothing: no counter is read and the history
//! holds no memory. Where the counters come from is the caller's
//! reading function; this module touches no platform interface.

pub(crate) mod counters;
pub(crate) mod memory;
pub(crate) mod sampler;
pub mod spread;
pub mod top;
pub mod tree;
pub(crate) mod volume;

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use wire::{Sample, Watched};

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

/// The two watcher counts and the history they gate.
#[derive(Debug, Default)]
pub struct Monitor {
    page_watchers: Arc<AtomicUsize>,
    summary_watchers: Arc<AtomicUsize>,
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

    /// Counts one more watcher of `watched` until the returned [`Watch`]
    /// is dropped.
    #[must_use]
    pub fn watch(&self, watched: Watched) -> Watch {
        let watchers = match watched {
            Watched::Everything => &self.page_watchers,
            Watched::Summary => &self.summary_watchers,
        };
        watchers.fetch_add(1, Ordering::Relaxed);
        Watch {
            watchers: Arc::clone(watchers),
        }
    }

    /// Called once a second. While somebody watches, calls `read` once,
    /// with [`Watched::Everything`] when anybody watches the whole page
    /// and [`Watched::Summary`] when only summaries are watched, and
    /// keeps its sample, dropping the oldest beyond [`CAPACITY`]; while
    /// nobody does, calls nothing and releases the history.
    pub fn tick(&mut self, read: impl FnOnce(Watched) -> Sample) {
        let Some(watched) = self.watched() else {
            self.history = VecDeque::new();
            return;
        };
        if self.history.len() == CAPACITY {
            self.history.pop_front();
        }
        self.history
            .reserve_exact(CAPACITY.saturating_sub(self.history.len()));
        self.history.push_back(read(watched));
    }

    /// Whether anybody holds a [`Watch`] right now.
    #[must_use]
    pub fn is_watched(&self) -> bool {
        self.watched().is_some()
    }

    /// The most anybody watches right now: the whole page when anybody
    /// does, else the summary when anybody watches that.
    fn watched(&self) -> Option<Watched> {
        let watching = |watchers: &AtomicUsize| watchers.load(Ordering::Relaxed) > 0;
        if watching(&self.page_watchers) {
            Some(Watched::Everything)
        } else if watching(&self.summary_watchers) {
            Some(Watched::Summary)
        } else {
            None
        }
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
