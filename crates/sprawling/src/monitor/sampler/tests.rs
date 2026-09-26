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

    beat(&monitor, &samples, || labelled(1));
    let unwatched = received.try_recv().ok();

    let watch = monitor.lock().unwrap().watch();
    beat(&monitor, &samples, || labelled(2));
    let watched = received.try_recv().ok();
    drop(watch);

    assert_eq!((unwatched, watched), (None, Some(labelled(2))));
}
