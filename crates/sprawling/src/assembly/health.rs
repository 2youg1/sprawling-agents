// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The accounting thread's health as counts other threads can read
//! (sprawling-SPEC.md 8-98).
//!
//! `relay` owns both counts: only it knows when a lane's append enters
//! the accounting thread's queue, when the accounting thread takes it
//! into a batch, and when that batch is durable. The monitor's sampler
//! reads them from here.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use wire::Sample;

/// Appends waiting for the accounting thread, and appends it has taken
/// whose disk barrier has not returned. A clone is another handle onto
/// the same counts.
///
/// Relaxed throughout: the two counts constrain nothing about each
/// other, and no decision reads them.
#[derive(Clone, Default)]
pub(crate) struct Health(Arc<Counts>);

#[derive(Default)]
struct Counts {
    queued: AtomicU64,
    unflushed: AtomicU64,
}

impl Health {
    /// One append is about to enter the accounting thread's queue.
    pub(crate) fn asked(&self) {
        self.0.queued.fetch_add(1, Ordering::Relaxed);
    }

    /// The append `asked` announced never entered the queue.
    pub(crate) fn withdrawn(&self) {
        lower(&self.0.queued, 1);
    }

    /// The accounting thread took `n` appends into one batch.
    pub(crate) fn taken(&self, n: u64) {
        lower(&self.0.queued, n);
        self.0.unflushed.fetch_add(n, Ordering::Relaxed);
    }

    /// The batch of `n` is durable and its answers are sent.
    pub(crate) fn answered(&self, n: u64) {
        lower(&self.0.unflushed, n);
    }

    /// `into`, with the two counts as they stand now.
    pub(crate) fn read(&self, into: Sample) -> Sample {
        Sample {
            ledger_queue_depth: self.0.queued.load(Ordering::Relaxed),
            durable_lag: self.0.unflushed.load(Ordering::Relaxed),
            ..into
        }
    }
}

/// Saturating: the two threads' updates can be read out of order for
/// an instant, and a reading of 0 is truer than one near `u64::MAX`.
fn lower(count: &AtomicU64, n: u64) {
    // The closure always returns `Some`, so the update always lands.
    match count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |now| {
        Some(now.saturating_sub(n))
    }) {
        Ok(_) | Err(_) => {}
    }
}
