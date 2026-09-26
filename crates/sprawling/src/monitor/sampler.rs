// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The thread that ticks the monitor once a second and sends each fresh
//! reading to the watching sessions (sprawling-SPEC.md 8-96).

use std::sync::{Mutex, PoisonError, Weak};
use std::time::Duration;

use kernel::{AxCode, AxError};
use tokio::sync::broadcast;

use super::counters::Counters;
use super::health::Health;
use super::{Monitor, Sample};

const BEAT: Duration = Duration::from_secs(1);

/// Starts the `sprawling-monitor` thread. It ends at the first beat
/// after the monitor it was handed has been dropped.
///
/// # Errors
/// `StorageFatal` when the thread cannot be started.
pub(crate) fn spawn_sampler(
    monitor: Weak<Mutex<Monitor>>,
    samples: broadcast::Sender<Sample>,
    volume: std::path::PathBuf,
    health: Health,
) -> Result<(), AxError> {
    std::thread::Builder::new()
        .name("sprawling-monitor".to_owned())
        .spawn(move || sample_until_dropped(&monitor, &samples, volume, &health))
        .map(drop)
        .map_err(|source| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the monitor's sampler",
                source.to_string(),
            )
            .with_recovery("check process thread limits")
        })
}

fn sample_until_dropped(
    monitor: &Weak<Mutex<Monitor>>,
    samples: &broadcast::Sender<Sample>,
    volume: std::path::PathBuf,
    health: &Health,
) {
    let mut counters: Option<Counters> = None;
    loop {
        std::thread::sleep(BEAT);
        let Some(monitor) = monitor.upgrade() else {
            return;
        };
        beat(&monitor, samples, |watched| {
            health.read(
                counters
                    .get_or_insert_with(|| Counters::open(volume.clone()))
                    .read(watched, BEAT),
            )
        });
        if !lock(&monitor).is_watched() {
            counters = None;
        }
    }
}

/// One beat: ticks the monitor and, when that read the counters, sends
/// the one reading it kept.
pub(crate) fn beat(
    monitor: &Mutex<Monitor>,
    samples: &broadcast::Sender<Sample>,
    read: impl FnOnce(super::Watched) -> Sample,
) {
    let mut fresh = None;
    lock(monitor).tick(|watched| *fresh.insert(read(watched)));
    if let Some(sample) = fresh {
        match samples.send(sample) {
            // No subscriber is not a failure: a session that stopped
            // watching between the tick and the send asked for nothing.
            Ok(_) | Err(broadcast::error::SendError(_)) => {}
        }
    }
}

/// A poisoned lock guards no half-written state: the watcher count is
/// an atomic and the history a whole `VecDeque`.
fn lock(monitor: &Mutex<Monitor>) -> std::sync::MutexGuard<'_, Monitor> {
    monitor.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "sampler/tests.rs"]
mod tests;
