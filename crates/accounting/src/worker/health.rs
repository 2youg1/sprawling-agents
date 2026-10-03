// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The accounting thread's health as counts other threads can read
//! (`crates/sprawling/Spec.lean` §8-98).
//!
//! `relay` owns both counts: only it knows when a lane's append enters
//! the accounting thread's queue, when the accounting thread takes it
//! into a batch, and when that batch is durable. The monitor's sampler
//! reads them from here. The two waits roadmap M2 reads at the relay,
//! the queue wait of each relay request and the time the accounting
//! thread slept on its queue, are kept here for the same reason.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use wire::Sample;

/// Appends waiting for the accounting thread, and appends it has taken
/// whose disk barrier has not returned. A clone is another handle onto
/// the same counts.
///
/// Relaxed throughout: the two counts constrain nothing about each
/// other, and no decision reads them.
#[derive(Clone, Default)]
pub struct Health(Arc<Counts>);

/// How many relay queue waits are kept, the most recent first to stay:
/// enough for a p999 over one arm of the throughput bench, and a bound
/// on what a long-lived city holds.
pub(crate) const RELAY_QUEUE_KEPT: usize = 65_536;

#[derive(Default)]
struct Counts {
    queued: AtomicU64,
    unflushed: AtomicU64,
    /// Microseconds the accounting thread slept on its queue.
    idle_us: AtomicU64,
    /// Microseconds each relay request waited between its lane queueing
    /// it and the accounting thread taking it, oldest first.
    relay_queue: Mutex<VecDeque<u64>>,
}

impl Health {
    /// One append is about to enter the accounting thread's queue.
    pub fn asked(&self) {
        self.0.queued.fetch_add(1, Ordering::Relaxed);
    }

    /// The append `asked` announced never entered the queue.
    pub(crate) fn withdrawn(&self) {
        lower(&self.0.queued, 1);
    }

    /// The accounting thread took `n` appends into one batch.
    pub fn taken(&self, n: u64) {
        lower(&self.0.queued, n);
        self.0.unflushed.fetch_add(n, Ordering::Relaxed);
    }

    /// The batch of `n` is durable and its answers are sent.
    pub fn answered(&self, n: u64) {
        lower(&self.0.unflushed, n);
    }

    /// The accounting thread took one relay request `waited` after its
    /// lane queued it.
    pub(crate) fn queue_waited(&self, waited: Duration) {
        let mut kept = self
            .0
            .relay_queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if kept.len() >= RELAY_QUEUE_KEPT {
            kept.pop_front();
        }
        kept.push_back(micros(waited));
    }

    /// The accounting thread slept `slept` on its queue.
    pub(crate) fn slept(&self, slept: Duration) {
        self.0.idle_us.fetch_add(micros(slept), Ordering::Relaxed);
    }

    /// The relay queue waits kept, in microseconds, oldest first.
    pub fn relay_queue_us(&self) -> Vec<u64> {
        self.0
            .relay_queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .copied()
            .collect()
    }

    /// Microseconds the accounting thread has slept on its queue since
    /// the gate opened; the busy share of a span is the rest of it.
    pub fn idle_us(&self) -> u64 {
        self.0.idle_us.load(Ordering::Relaxed)
    }

    /// `into`, with the two counts as they stand now.
    pub fn read(&self, into: Sample) -> Sample {
        Sample {
            ledger_queue_depth: self.0.queued.load(Ordering::Relaxed),
            durable_lag: self.0.unflushed.load(Ordering::Relaxed),
            ..into
        }
    }
}

/// Whole microseconds, saturating: a wait past `u64::MAX` microseconds
/// is not a reading anybody takes.
fn micros(span: Duration) -> u64 {
    u64::try_from(span.as_micros()).unwrap_or(u64::MAX)
}

/// Saturating: the two threads' updates can be read out of order for
/// an instant, and a reading of 0 is truer than one near `u64::MAX`.
fn lower(count: &AtomicU64, n: u64) {
    // The closure always returns `Some`, so the update always lands.
    match count.try_update(Ordering::Relaxed, Ordering::Relaxed, |now| {
        Some(now.saturating_sub(n))
    }) {
        Ok(_) | Err(_) => {}
    }
}
