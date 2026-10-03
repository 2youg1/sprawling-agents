// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The thread that ticks the monitor once a second and sends each fresh
//! reading to the watching sessions, and notes this process's private
//! bytes on every memory beat in between (`crates/sprawling/spec/Monitor.lean` §8-96).

use std::sync::{Mutex, PoisonError, Weak};
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError};
use tokio::sync::broadcast;

use super::counters::Counters;
use super::{Monitor, Sample};
use accounting::worker::health::Health;

/// How often private bytes are read while somebody watches: the memory
/// beat of `crates/sprawling/spec/Serving/Memory.lean`'s measuring plan.
const MEMORY_BEAT: Duration = Duration::from_millis(100);

/// Memory beats in one beat of the monitor.
const MEMORY_BEATS_PER_BEAT: u32 = 10;

const BEAT: Duration = MEMORY_BEAT.saturating_mul(MEMORY_BEATS_PER_BEAT);

/// What a beat adds to the counters it read: the accounting queue's two
/// counts (`crates/sprawling/spec/Accounting/Worker.lean` §8-98), the view fold's backlog (8-123), and
/// how long the beat before this one took to read (8-129-6).
pub(crate) struct Gauges<B> {
    health: Health,
    backlog: B,
    /// The sampler's own monotonic clock; a script in a test.
    clock: fn() -> Instant,
    last_read: Duration,
}

impl<B: Fn() -> u64> Gauges<B> {
    pub(crate) fn new(health: Health, backlog: B, clock: fn() -> Instant) -> Self {
        Gauges {
            health,
            backlog,
            clock,
            last_read: Duration::ZERO,
        }
    }

    /// Reads through `read`, timed by the sampler's own clock, and fills
    /// in what the counters cannot know. The read's own time is reported
    /// on the next beat, because this beat's sample is written before the
    /// read is over.
    pub(crate) fn sample(&mut self, read: impl FnOnce() -> Sample) -> Sample {
        let started = (self.clock)();
        let counted = read();
        let sample = self.health.read(Sample {
            view_backlog: (self.backlog)(),
            read_nanos: u64::try_from(self.last_read.as_nanos()).unwrap_or(u64::MAX),
            ..counted
        });
        self.last_read = (self.clock)().saturating_duration_since(started);
        sample
    }
}

/// Starts the `sprawling-monitor` thread. It ends at the first beat
/// after the monitor it was handed has been dropped.
///
/// # Errors
/// `StorageFatal` when the thread cannot be started.
pub(crate) fn spawn_sampler(
    monitor: Weak<Mutex<Monitor>>,
    samples: broadcast::Sender<Sample>,
    volume: std::path::PathBuf,
    mut gauges: Gauges<impl Fn() -> u64 + Send + 'static>,
) -> Result<(), AxError> {
    std::thread::Builder::new()
        .name("sprawling-monitor".to_owned())
        .spawn(move || sample_until_dropped(&monitor, &samples, volume, &mut gauges))
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
    gauges: &mut Gauges<impl Fn() -> u64>,
) {
    let mut counters: Option<Counters> = None;
    loop {
        std::thread::sleep(MEMORY_BEAT);
        (1..MEMORY_BEATS_PER_BEAT).for_each(|_| {
            if let Some(open) = counters.as_mut() {
                open.note_private();
            }
            std::thread::sleep(MEMORY_BEAT);
        });
        let Some(monitor) = monitor.upgrade() else {
            return;
        };
        beat(&monitor, samples, |watched| {
            gauges.sample(|| {
                counters
                    .get_or_insert_with(|| Counters::open(volume.clone()))
                    .read(watched, BEAT)
            })
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
