// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Nobody watching reads no counter; the history keeps the last 300.

#![allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::cell::Cell;

use super::{CAPACITY, Monitor, Sample};

fn labelled(n: u64) -> Sample {
    Sample {
        core_cpu_permille: n,
        ..Sample::default()
    }
}

#[test]
fn nobody_watching_reads_no_counter_and_keeps_no_history() {
    let reads = Cell::new(0_u32);
    let read = || {
        reads.set(reads.get() + 1);
        labelled(1)
    };
    let mut monitor = Monitor::new();
    monitor.tick(read);
    monitor.tick(read);
    assert_eq!((reads.get(), monitor.history().count()), (0, 0));

    let watch = monitor.watch();
    monitor.tick(read);
    assert_eq!((reads.get(), monitor.history().count()), (1, 1));

    drop(watch);
    monitor.tick(read);
    assert_eq!((reads.get(), monitor.history().count()), (1, 0));
    assert_eq!(monitor.history.capacity(), 0);
}

#[test]
fn history_keeps_the_last_300_samples_oldest_first() {
    let mut monitor = Monitor::new();
    let _watch = monitor.watch();
    for n in 0..=300 {
        monitor.tick(|| labelled(n));
    }
    let kept: Vec<u64> = monitor.history().map(|s| s.core_cpu_permille).collect();
    assert_eq!(kept, (1..=300).collect::<Vec<u64>>());
    assert_eq!(monitor.history.capacity(), CAPACITY);
}
