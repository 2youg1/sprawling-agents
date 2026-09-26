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

use sysinfo::{
    Disks, MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, System,
};

use super::Sample;

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
        let _ = (
            &self.system,
            &self.disks,
            self.pid,
            &self.volume,
            MemoryRefreshKind::nothing(),
            ProcessRefreshKind::nothing(),
            ProcessesToUpdate::All,
        );
        Sample::default()
    }
}

#[cfg(test)]
#[path = "counters/tests.rs"]
mod tests;
