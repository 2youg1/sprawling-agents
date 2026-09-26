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

/// This process's handle, the machine's handles while the whole page is
/// watched, and the volume whose free space is read.
pub(crate) struct Counters {
    own: OwnProcess,
    machine: Option<Machine>,
    volume: PathBuf,
}

/// The `sysinfo` handles, which cost milliseconds a reading; only a
/// watcher of the whole page pays for them.
struct Machine {
    system: System,
    disks: Disks,
}

impl Counters {
    /// Opens this process's handle; the machine's wait for the first
    /// reading of [`Watched::Everything`]. The first reading has no
    /// earlier one to compare with, so both CPU figures read 0.
    pub(crate) fn open(volume: PathBuf) -> Self {
        Self {
            own: OwnProcess::new(),
            machine: None,
            volume,
        }
    }

    /// Reads one sample; `elapsed` is the wall time since the previous
    /// reading. [`Watched::Summary`] reads this process alone and closes
    /// the machine's handles. The core-health fields are not read here
    /// and stay 0 (8-92, current state).
    pub(crate) fn read(&mut self, watched: Watched, elapsed: Duration) -> Sample {
        let core = self.own.read(elapsed);
        let own = Sample {
            core_cpu_permille: core.cpu_permille,
            core_private_bytes: core.private_bytes,
            core_working_set_bytes: core.working_set_bytes,
            core_read_bytes: core.read_bytes,
            core_written_bytes: core.written_bytes,
            ..Sample::default()
        };
        match watched {
            Watched::Summary => {
                self.machine = None;
                own
            }
            Watched::Everything => self
                .machine
                .get_or_insert_with(Machine::open)
                .read(&self.volume, own),
        }
    }
}

impl Machine {
    fn open() -> Self {
        Self {
            system: System::new(),
            disks: Disks::new_with_refreshed_list(),
        }
    }

    /// Refreshes the machine's counters into `own`.
    fn read(&mut self, volume: &std::path::Path, own: Sample) -> Sample {
        self.system.refresh_cpu_usage();
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        self.disks.refresh(true);
        Sample {
            machine_cpu_permille: permille(self.system.global_cpu_usage()),
            machine_available_bytes: self.system.available_memory(),
            volume_free_bytes: super::volume::space(&self.disks, volume)
                .map_or(0, |space| space.free_bytes),
            ..own
        }
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
