// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A beat sends the one reading it took, and only while somebody watches.

#![allow(clippy::unwrap_used, reason = "test code")]

use std::sync::Mutex;

use tokio::sync::broadcast;

use super::beat;
use crate::monitor::{Monitor, Sample};

fn labelled(n: u64) -> Sample {
    Sample {
        queued_runs: n,
        ..Sample::default()
    }
}

#[test]
fn a_watched_beat_sends_the_reading_it_took_and_an_unwatched_one_sends_nothing() {
    let monitor = Mutex::new(Monitor::new());
    let (samples, mut received) = broadcast::channel(4);

    beat(&monitor, &samples, |_| labelled(1));
    let unwatched = received.try_recv().ok();

    let watch = monitor
        .lock()
        .unwrap()
        .watch(crate::monitor::Watched::Everything);
    beat(&monitor, &samples, |_| labelled(2));
    let watched = received.try_recv().ok();
    drop(watch);

    assert_eq!((unwatched, watched), (None, Some(labelled(2))));
}

/// A clock that moves 1.5 microseconds every time it is read.
fn ticking() -> std::time::Instant {
    static BASE: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let reads = READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    BASE.get_or_init(crate::serving::standing::monotonic_now)
        .checked_add(std::time::Duration::from_nanos(reads.saturating_mul(1_500)))
        .unwrap()
}

/// Each beat carries the views' backlog as it stands, and the time the
/// beat before it took to read, by the sampler's own clock; the first
/// beat has no beat before it.
#[test]
fn a_beat_carries_the_backlog_and_the_time_the_beat_before_it_took_to_read() {
    let mut gauges =
        super::Gauges::new(accounting::worker::health::Health::default(), || 7, ticking);
    let first = gauges.sample(Sample::default);
    let second = gauges.sample(Sample::default);
    assert_eq!(
        (first.view_backlog, first.read_nanos, second.read_nanos),
        (7, 0, 1_500)
    );
}
