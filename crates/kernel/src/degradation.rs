// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Degrade truthfully: which resource is the bottleneck, the readings
//! that show it, and what the person can do about it.
//! Readings are sampled by the caller; this module only decides. The one
//! state that stops the city taking on new work is a disk close to full,
//! and it sheds through [`crate::backpressure::Admission`], never a second
//! admission.

use std::time::Duration;

use crate::backpressure::{Admission, ShedReason};

/// How many commit floors the durable watermark may lag before the disk
/// counts as slow. Relative, so a spinning disk whose fsync is simply
/// slow is not reported as degraded.
pub const DISK_SLOW_FACTOR: u32 = 8;
/// The lag below which the disk never counts as slow, so a fast device's
/// jitter of a few commit floors does not flap the state.
pub const DISK_SLOW_MIN: Duration = Duration::from_millis(20);
/// One 60 Hz frame: the scheduling delay a person starts to see.
pub const CPU_SATURATED_DELAY: Duration = Duration::from_micros(16_667);
/// The free-space floor is this fraction of the volume, before clamping.
pub const FREE_SPACE_FLOOR_DIVISOR: u64 = 50;
/// Room for the next snapshot and git's objects even on a small volume.
pub const FREE_SPACE_FLOOR_MIN: u64 = 256 * 1024 * 1024;
/// A large volume does not hold back tens of gigabytes as its floor.
pub const FREE_SPACE_FLOOR_MAX: u64 = 4 * 1024 * 1024 * 1024;

/// One sample of the resources the harness depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceReadings {
    /// How long the oldest appended-but-not-durable event has waited.
    pub durable_lag: Duration,
    /// The measured fsync median of the city's volume: this step's floor.
    pub commit_floor: Duration,
    pub free_bytes: u64,
    pub volume_bytes: u64,
    /// Runs waiting because available memory would not hold one more.
    pub queued_runs: u32,
    /// The accounting thread's median wake-to-run delay.
    pub schedule_delay: Duration,
}

/// A resource that is the bottleneck, carrying the readings that decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Degradation {
    DiskSlow {
        durable_lag: Duration,
        commit_floor: Duration,
    },
    DiskLow {
        free_bytes: u64,
        floor_bytes: u64,
    },
    MemoryTight {
        queued_runs: u32,
    },
    CpuSaturated {
        schedule_delay: Duration,
    },
}

/// What the person can do to end a degradation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    ReduceDiskLoad,
    FreeDiskSpace { at_least_bytes: u64 },
    FreeMemory,
    ReduceCpuLoad,
}

impl Degradation {
    pub fn recovery(&self) -> Recovery {
        match *self {
            Degradation::DiskSlow { .. } => Recovery::ReduceDiskLoad,
            Degradation::DiskLow {
                free_bytes,
                floor_bytes,
            } => Recovery::FreeDiskSpace {
                at_least_bytes: floor_bytes.saturating_sub(free_bytes),
            },
            Degradation::MemoryTight { .. } => Recovery::FreeMemory,
            Degradation::CpuSaturated { .. } => Recovery::ReduceCpuLoad,
        }
    }
}

/// Every degradation the readings show, in declaration order, without
/// allocating.
pub fn assess(readings: &ResourceReadings) -> impl Iterator<Item = Degradation> {
    [
        disk_slow(readings),
        disk_low(readings),
        memory_tight(readings),
        cpu_saturated(readings),
    ]
    .into_iter()
    .flatten()
}

/// Whether the city takes on new work. A disk below its free-space floor
/// sheds; every other degradation admits, because it only slows work down.
pub fn admit_work(readings: &ResourceReadings) -> Admission {
    match disk_low(readings) {
        Some(_) => Admission::Shed {
            reason: ShedReason::DiskLow,
        },
        None => Admission::Admit,
    }
}

/// The free-space floor a volume of `volume_bytes` derives.
pub fn free_space_floor(volume_bytes: u64) -> u64 {
    (volume_bytes / FREE_SPACE_FLOOR_DIVISOR)
        .max(FREE_SPACE_FLOOR_MIN)
        .min(FREE_SPACE_FLOOR_MAX)
}

fn disk_slow(r: &ResourceReadings) -> Option<Degradation> {
    let relative_line = r.commit_floor.saturating_mul(DISK_SLOW_FACTOR);
    (r.durable_lag > relative_line.max(DISK_SLOW_MIN)).then_some(Degradation::DiskSlow {
        durable_lag: r.durable_lag,
        commit_floor: r.commit_floor,
    })
}

fn disk_low(r: &ResourceReadings) -> Option<Degradation> {
    let floor_bytes = free_space_floor(r.volume_bytes);
    (r.free_bytes < floor_bytes).then_some(Degradation::DiskLow {
        free_bytes: r.free_bytes,
        floor_bytes,
    })
}

fn memory_tight(r: &ResourceReadings) -> Option<Degradation> {
    (r.queued_runs > 0).then_some(Degradation::MemoryTight {
        queued_runs: r.queued_runs,
    })
}

fn cpu_saturated(r: &ResourceReadings) -> Option<Degradation> {
    (r.schedule_delay > CPU_SATURATED_DELAY).then_some(Degradation::CpuSaturated {
        schedule_delay: r.schedule_delay,
    })
}

#[cfg(test)]
mod tests;
