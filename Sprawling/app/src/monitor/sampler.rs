// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError};
use tokio::sync::broadcast;

use super::beat::Beat;
use super::counters::Counters;
use super::{Monitor, Sample};
use accounting::worker::health::Health;

const MEMORY_BEATS_PER_BEAT: u32 = 10;

pub(crate) struct Gauges<B> {
    health: Health,
    backlog: B,

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

pub(crate) fn spawn_sampler(
    monitor: Weak<Mutex<Monitor>>,
    samples: broadcast::Sender<Sample>,
    volume: std::path::PathBuf,
    mut gauges: Gauges<impl Fn() -> u64 + Send + 'static>,
) -> Result<Arc<Beat>, AxError> {
    let beat = Arc::new(Beat::open(&volume));
    let held = Arc::clone(&beat);
    std::thread::Builder::new()
        .name("sprawling-monitor".to_owned())
        .spawn(move || {
            sample_until_dropped(
                &monitor,
                &samples,
                &Measured { volume, beat: held },
                &mut gauges,
            )
        })
        .map(|_| beat)
        .map_err(|source| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the monitor's sampler",
                source.to_string(),
            )
            .with_recovery("check process thread limits")
        })
}

struct Measured {
    volume: std::path::PathBuf,
    beat: Arc<Beat>,
}

fn sample_until_dropped(
    monitor: &Weak<Mutex<Monitor>>,
    samples: &broadcast::Sender<Sample>,
    measured: &Measured,
    gauges: &mut Gauges<impl Fn() -> u64>,
) {
    let Measured { volume, beat } = measured;
    let mut counters: Option<Counters> = None;
    loop {
        std::thread::sleep(beat.duration());
        (1..MEMORY_BEATS_PER_BEAT).for_each(|_| {
            if let Some(open) = counters.as_mut() {
                open.note_private();
            }
            std::thread::sleep(beat.duration());
        });
        let Some(monitor) = monitor.upgrade() else {
            return;
        };
        let sampled_at = beat.now();
        let elapsed = beat.duration().saturating_mul(MEMORY_BEATS_PER_BEAT);
        self::beat(&monitor, samples, |watched| {
            gauges.sample(|| Sample {
                beat_ms: u64::from(sampled_at.ms()),
                ..counters
                    .get_or_insert_with(|| Counters::open(volume.clone()))
                    .read(watched, elapsed)
            })
        });
        if !lock(&monitor).is_watched() {
            counters = None;
        }
    }
}

pub(crate) fn beat(
    monitor: &Mutex<Monitor>,
    samples: &broadcast::Sender<Sample>,
    read: impl FnOnce(super::Watched) -> Sample,
) {
    let mut fresh = None;
    lock(monitor).tick(|watched| *fresh.insert(read(watched)));
    if let Some(sample) = fresh {
        match samples.send(sample) {

            Ok(_) | Err(broadcast::error::SendError(_)) => {}
        }
    }
}

fn lock(monitor: &Mutex<Monitor>) -> std::sync::MutexGuard<'_, Monitor> {
    monitor.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "sampler/tests.rs"]
mod tests;
