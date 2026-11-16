// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;
use std::time::Duration;

use sysinfo::{Disks, System};

use super::{Sample, Watched};
use own_process::OwnProcess;

pub(crate) mod own_process;

pub(crate) struct Counters {
    own: OwnProcess,
    machine: Option<Machine>,
    volume: PathBuf,
    reads: Reads,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Reads {
    pub(crate) own: u64,
    pub(crate) machine: u64,
    pub(crate) table: u64,
}

struct Machine {
    system: System,
    disks: Disks,
}

impl Counters {
    pub(crate) fn open(volume: PathBuf) -> Self {
        Self {
            own: OwnProcess::new(),
            machine: None,
            volume: super::volume::resolved(&volume).unwrap_or(volume),
            reads: Reads::default(),
        }
    }

    pub(crate) fn read(&mut self, watched: Watched, elapsed: Duration) -> Sample {
        let core = self.own.read(elapsed);
        self.reads.own = self.reads.own.saturating_add(1);
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
            Watched::Everything => {
                self.reads.machine = self.reads.machine.saturating_add(1);
                self.machine
                    .get_or_insert_with(Machine::open)
                    .read(&self.volume, own)
            }
        }
    }

    pub(crate) fn note_private(&mut self) {
        self.own.note_private();
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the count gate's tests read it (`Sprawling/app/spec/Main.lean` §8-129-3); a production reading only counts"
        )
    )]
    pub(crate) fn reads(&self) -> Reads {
        self.reads
    }
}

impl Machine {
    fn open() -> Self {
        Self {
            system: System::new(),
            disks: Disks::new_with_refreshed_list(),
        }
    }

    fn read(&mut self, volume: &std::path::Path, own: Sample) -> Sample {
        self.system.refresh_cpu_usage();
        let memory = super::memory::refreshed(&mut self.system);
        self.disks.refresh(true);
        Sample {
            machine_cpu_permille: permille(self.system.global_cpu_usage()),
            machine_available_bytes: memory.available,
            volume_free_bytes: super::volume::space(&self.disks, volume)
                .map_or(0, |space| space.free_bytes),
            ..own
        }
    }
}

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
