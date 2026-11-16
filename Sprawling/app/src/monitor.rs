// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) mod beat;
pub(crate) mod counters;
pub mod memory;
pub(crate) mod sampler;
pub mod spread;
pub mod top;
pub mod tree;
pub(crate) mod volume;

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use wire::{Sample, Watched};

pub const CAPACITY: usize = 300;

pub const HISTORY_BUDGET: usize = 64 * 1024;

const _: () = assert!(
    match CAPACITY.checked_mul(size_of::<Sample>()) {
        Some(bytes) => bytes <= HISTORY_BUDGET,
        None => false,
    },
    "the monitor history outgrew HISTORY_BUDGET"
);

#[derive(Debug, Default)]
pub struct Monitor {
    page_watchers: Arc<AtomicUsize>,
    summary_watchers: Arc<AtomicUsize>,
    history: VecDeque<Sample>,
}

#[derive(Debug)]
pub struct Watch {
    watchers: Arc<AtomicUsize>,
}

impl Monitor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

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

    #[must_use]
    pub fn is_watched(&self) -> bool {
        self.watched().is_some()
    }

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
