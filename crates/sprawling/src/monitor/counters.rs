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
use std::time::Duration;

use sysinfo::{Disks, MemoryRefreshKind, System};

use super::{Sample, Watched};
use own_process::OwnProcess;

pub(crate) mod own_process;

/// The open platform handles, and the volume whose free space is read.
pub(crate) struct Counters {
    system: System,
    disks: Disks,
    own: OwnProcess,
    volume: PathBuf,
}

impl Counters {
    /// Opens the handles. The first reading has no earlier one to
    /// compare with, so both CPU figures read 0.
    pub(crate) fn open(volume: PathBuf) -> Self {
        Self {
            system: System::new(),
            disks: Disks::new_with_refreshed_list(),
            own: OwnProcess::new(),
            volume,
        }
    }

    /// Refreshes every counter and reads one sample; `elapsed` is the
    /// wall time since the previous reading. The core-health
    /// fields and this process's read and written bytes are not read
    /// here and stay 0 (8-92, decision 1 and current state).
    pub(crate) fn read(&mut self, _watched: Watched, elapsed: Duration) -> Sample {
        self.system.refresh_cpu_usage();
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        self.disks.refresh(true);
        let core = self.own.read(elapsed);
        Sample {
            core_cpu_permille: core.cpu_permille,
            core_private_bytes: core.private_bytes,
            core_working_set_bytes: core.working_set_bytes,
            machine_cpu_permille: permille(self.system.global_cpu_usage()),
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

/// A load in percent, as permille.
#[expect(
    clippy::as_conversions,
    reason = "sysinfo reports load only as f32; it is clamped to 0..=1000 first, so the cast is exact to the permille"
)]
fn permille(percent: f32) -> u64 {
    (percent * 10.0).clamp(0.0, 1000.0) as u64
}

#[cfg(test)]
#[path = "counters/tests.rs"]
mod tests;
