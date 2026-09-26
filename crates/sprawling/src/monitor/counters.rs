// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The platform counters one [`Sample`] is read from: this process, the
//! machine, and the volume the city lives on (sprawling-SPEC.md 8-92).
//!
//! A `Counters` exists only while somebody watches; the sampler drops it
//! when the last watcher leaves, so its handles and process table are
//! not resident in an unwatched city.

use std::path::PathBuf;

use sysinfo::{Disks, MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::Sample;

pub(crate) mod own_process;

/// The open platform handles, and the volume whose free space is read.
pub(crate) struct Counters {
    system: System,
    disks: Disks,
    pid: Pid,
    volume: PathBuf,
}

impl Counters {
    /// Opens the handles. The first reading has no earlier one to
    /// compare with, so both CPU figures read 0.
    pub(crate) fn open(volume: PathBuf) -> Self {
        Self {
            system: System::new(),
            disks: Disks::new_with_refreshed_list(),
            pid: Pid::from_u32(std::process::id()),
            volume,
        }
    }

    /// Refreshes every counter and reads one sample. The core-health
    /// fields are not read here yet and stay 0 (8-92, current state).
    pub(crate) fn read(&mut self) -> Sample {
        self.system.refresh_cpu_usage();
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage(),
        );
        self.disks.refresh(true);
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        let core = self.system.process(self.pid);
        let disk = core.map(sysinfo::Process::disk_usage);
        Sample {
            core_cpu_permille: core.map_or(0, |process| permille(process.cpu_usage(), cores)),
            core_private_bytes: core.map_or(0, sysinfo::Process::virtual_memory),
            core_working_set_bytes: core.map_or(0, sysinfo::Process::memory),
            core_read_bytes: disk.map_or(0, |usage| usage.total_read_bytes),
            core_written_bytes: disk.map_or(0, |usage| usage.total_written_bytes),
            machine_cpu_permille: permille(self.system.global_cpu_usage(), 1),
            machine_available_bytes: self.system.available_memory(),
            volume_free_bytes: self.volume_free_bytes(),
            ..Sample::default()
        }
    }

    /// The free space of the disk whose mount point is the longest
    /// prefix of the city's path.
    fn volume_free_bytes(&self) -> u64 {
        self.disks
            .list()
            .iter()
            .filter(|disk| self.volume.starts_with(disk.mount_point()))
            .max_by_key(|disk| disk.mount_point().components().count())
            .map_or(0, sysinfo::Disk::available_space)
    }
}

/// A load in percent spread over `cores`, as permille of the whole.
#[expect(
    clippy::as_conversions,
    reason = "sysinfo reports load only as f32; it is clamped to 0..=1000 first, so the cast is exact to the permille"
)]
fn permille(percent: f32, cores: usize) -> u64 {
    let spread = u16::try_from(cores).map_or(f32::from(u16::MAX), f32::from);
    (percent / spread * 10.0).clamp(0.0, 1000.0) as u64
}

#[cfg(test)]
#[path = "counters/tests.rs"]
mod tests;
