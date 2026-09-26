// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This process's own counters, read through its own handle rather than
//! the whole process table, so one reading costs about a microsecond
//! (sprawling-SPEC.md 8-92).

use std::time::Duration;

use cpu_time::ProcessTime;

/// The five figures of this process one sample carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OwnReading {
    pub(crate) cpu_permille: u64,
    pub(crate) private_bytes: u64,
    pub(crate) working_set_bytes: u64,
    pub(crate) read_bytes: u64,
    pub(crate) written_bytes: u64,
}

/// The previous reading's CPU time, which the next reading's CPU share
/// is measured against.
pub(crate) struct OwnProcess {
    previous: Option<Duration>,
}

impl OwnProcess {
    pub(crate) fn new() -> Self {
        Self { previous: None }
    }

    /// Reads this process once; `elapsed` is the wall time since the
    /// previous reading. The first reading has nothing to compare with,
    /// so its CPU reads 0; a figure the platform refuses reads 0.
    pub(crate) fn read(&mut self, elapsed: Duration) -> OwnReading {
        let cpu = ProcessTime::try_now().ok().map(|time| time.as_duration());
        let cpu_permille = match (cpu, self.previous) {
            (Some(spent), Some(spent_before)) => {
                permille(spent.saturating_sub(spent_before), elapsed)
            }
            (Some(_) | None, None) | (None, Some(_)) => 0,
        };
        self.previous = cpu;
        let memory = memory_stats::memory_stats();
        let storage = storage_bytes();
        OwnReading {
            cpu_permille,
            private_bytes: memory.map_or(0, |stats| widen(stats.virtual_mem)),
            working_set_bytes: memory.map_or(0, |stats| widen(stats.physical_mem)),
            read_bytes: storage.read,
            written_bytes: storage.written,
        }
    }
}

/// The bytes this process has read from and written to storage.
#[derive(Default)]
struct StorageBytes {
    read: u64,
    written: u64,
}

/// Linux keeps them in `/proc/self/io`, a plain file std reads safely;
/// a line the kernel does not offer reads 0.
#[cfg(target_os = "linux")]
fn storage_bytes() -> StorageBytes {
    let Ok(io) = std::fs::read_to_string("/proc/self/io") else {
        return StorageBytes::default();
    };
    let field = |name: &str| {
        io.lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0)
    };
    StorageBytes {
        read: field("read_bytes:"),
        written: field("write_bytes:"),
    }
}

/// Elsewhere the counter is reachable only through `unsafe`, which this
/// workspace forbids, so both read 0 (sprawling-SPEC.md 8-92, decision 1).
#[cfg(not(target_os = "linux"))]
fn storage_bytes() -> StorageBytes {
    StorageBytes::default()
}

/// CPU time spent over wall time elapsed, spread over every core, as
/// permille of the whole machine, clamped to `0..=1000`.
fn permille(spent: Duration, elapsed: Duration) -> u64 {
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let capacity = elapsed
        .as_nanos()
        .saturating_mul(u128::try_from(cores).unwrap_or(u128::MAX));
    spent
        .as_nanos()
        .saturating_mul(1000)
        .checked_div(capacity)
        .map_or(0, |share| u64::try_from(share.min(1000)).unwrap_or(1000))
}

fn widen(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[path = "own_process/tests.rs"]
mod tests;
