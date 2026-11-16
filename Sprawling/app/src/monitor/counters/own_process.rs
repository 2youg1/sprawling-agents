// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use cpu_time::ProcessTime;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OwnReading {
    pub(crate) cpu_permille: u64,
    pub(crate) private_bytes: u64,
    pub(crate) working_set_bytes: u64,
    pub(crate) read_bytes: u64,
    pub(crate) written_bytes: u64,
}

pub(crate) struct OwnProcess {
    previous: Option<Duration>,
    peak_private: u64,
}

impl OwnProcess {
    pub(crate) fn new() -> Self {
        Self {
            previous: None,
            peak_private: 0,
        }
    }

    pub(crate) fn note_private(&mut self) {
        let now = private_bytes(memory_stats::memory_stats().as_ref());
        self.peak_private = self.peak_private.max(now);
    }

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
            private_bytes: private_bytes(memory.as_ref())
                .max(std::mem::take(&mut self.peak_private)),
            working_set_bytes: memory.map_or(0, |stats| widen(stats.physical_mem)),
            read_bytes: storage.read,
            written_bytes: storage.written,
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn private_bytes(memory: Option<&memory_stats::MemoryStats>) -> u64 {
    memory.map_or(0, |stats| widen(stats.virtual_mem))
}

#[cfg(target_os = "linux")]
fn private_bytes(_memory: Option<&memory_stats::MemoryStats>) -> u64 {
    let read = |path: &str| std::fs::read_to_string(path).ok();
    read("/proc/self/smaps_rollup")
        .and_then(|rollup| kib_sum(&rollup, &["Private_Clean:", "Private_Dirty:"]))
        .or_else(|| read("/proc/self/status").and_then(|status| kib_sum(&status, &["RssAnon:"])))
        .unwrap_or(0)
}

#[cfg(any(target_os = "linux", test))]
fn kib_sum(text: &str, fields: &[&str]) -> Option<u64> {
    text.lines()
        .filter_map(|line| {
            fields
                .iter()
                .find_map(|field| line.strip_prefix(field))
                .and_then(|value| value.trim().strip_suffix("kB"))
                .and_then(|kib| kib.trim().parse::<u64>().ok())
        })
        .fold(None, |sum: Option<u64>, kib| {
            Some(sum.unwrap_or(0).saturating_add(kib.saturating_mul(1024)))
        })
}

#[derive(Default)]
struct StorageBytes {
    read: u64,
    written: u64,
}

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

#[cfg(not(target_os = "linux"))]
fn storage_bytes() -> StorageBytes {
    StorageBytes::default()
}

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
