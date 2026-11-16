// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

pub const DISK_SLOW_FACTOR: u32 = 8;

pub const DISK_SLOW_MIN: Duration = Duration::from_millis(20);

pub const CPU_SATURATED_DELAY: Duration = Duration::from_micros(16_667);

pub const FREE_SPACE_FLOOR_DIVISOR: u64 = 50;

pub const FREE_SPACE_FLOOR_MIN: u64 = 256 * 1024 * 1024;

pub const FREE_SPACE_FLOOR_MAX: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceReadings {
    pub durable_lag: Duration,

    pub commit_floor: Duration,
    pub volume: VolumeSpace,

    pub queued_runs: u32,

    pub schedule_delay: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

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

pub fn assess(readings: &ResourceReadings) -> impl Iterator<Item = Degradation> {
    [
        disk_slow(readings),
        disk_low(readings.volume),
        memory_tight(readings),
        cpu_saturated(readings),
    ]
    .into_iter()
    .flatten()
}

pub fn admit_work(volume: VolumeSpace) -> Result<(), Degradation> {
    disk_low(volume).map_or(Ok(()), Err)
}

const _: () = assert!(FREE_SPACE_FLOOR_MIN <= FREE_SPACE_FLOOR_MAX);

pub fn free_space_floor(volume_bytes: u64) -> u64 {
    (volume_bytes / FREE_SPACE_FLOOR_DIVISOR).clamp(FREE_SPACE_FLOOR_MIN, FREE_SPACE_FLOOR_MAX)
}

fn disk_slow(r: &ResourceReadings) -> Option<Degradation> {
    let relative_line = r.commit_floor.saturating_mul(DISK_SLOW_FACTOR);
    (r.durable_lag > relative_line.max(DISK_SLOW_MIN)).then_some(Degradation::DiskSlow {
        durable_lag: r.durable_lag,
        commit_floor: r.commit_floor,
    })
}

fn disk_low(volume: VolumeSpace) -> Option<Degradation> {
    let floor_bytes = free_space_floor(volume.total_bytes);
    (volume.free_bytes < floor_bytes).then_some(Degradation::DiskLow {
        free_bytes: volume.free_bytes,
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
