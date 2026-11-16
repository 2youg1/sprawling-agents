// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use wire::Sample;

#[derive(Clone, Default)]
pub struct Health(Arc<Counts>);

pub(crate) const RELAY_QUEUE_KEPT: usize = 65_536;

#[derive(Default)]
struct Counts {
    queued: AtomicU64,
    unflushed: AtomicU64,

    idle_us: AtomicU64,

    relay_queue: Mutex<VecDeque<u64>>,

    lane_wait: Mutex<VecDeque<u64>>,
}

impl Health {

    pub fn asked(&self) {
        self.0.queued.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn withdrawn(&self) {
        lower(&self.0.queued, 1);
    }

    pub fn taken(&self, n: u64) {
        lower(&self.0.queued, n);
        self.0.unflushed.fetch_add(n, Ordering::Relaxed);
    }

    pub fn answered(&self, n: u64) {
        lower(&self.0.unflushed, n);
    }

    pub(crate) fn queue_waited(&self, waited: Duration) {
        keep(&self.0.relay_queue, waited);
    }

    pub(crate) fn lane_waited(&self, waited: Duration) {
        keep(&self.0.lane_wait, waited);
    }

    pub fn lane_wait_us(&self) -> Vec<u64> {
        kept(&self.0.lane_wait)
    }

    pub(crate) fn slept(&self, slept: Duration) {
        self.0.idle_us.fetch_add(micros(slept), Ordering::Relaxed);
    }

    pub fn relay_queue_us(&self) -> Vec<u64> {
        kept(&self.0.relay_queue)
    }

    pub fn idle_us(&self) -> u64 {
        self.0.idle_us.load(Ordering::Relaxed)
    }

    pub fn read(&self, into: Sample) -> Sample {
        Sample {
            ledger_queue_depth: self.0.queued.load(Ordering::Relaxed),
            durable_lag: self.0.unflushed.load(Ordering::Relaxed),
            ..into
        }
    }
}

fn micros(span: Duration) -> u64 {
    u64::try_from(span.as_micros()).unwrap_or(u64::MAX)
}

fn lower(count: &AtomicU64, n: u64) {

    match count.try_update(Ordering::Relaxed, Ordering::Relaxed, |now| {
        Some(now.saturating_sub(n))
    }) {
        Ok(_) | Err(_) => {}
    }
}

fn keep(waits: &Mutex<VecDeque<u64>>, waited: Duration) {
    let mut kept = waits.lock().unwrap_or_else(PoisonError::into_inner);
    if kept.len() >= RELAY_QUEUE_KEPT {
        kept.pop_front();
    }
    kept.push_back(micros(waited));
}

fn kept(waits: &Mutex<VecDeque<u64>>) -> Vec<u64> {
    waits
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .copied()
        .collect()
}
