// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This process's own counters, read through its own handle rather than
//! the whole process table, so one reading costs about a microsecond
//! (sprawling-SPEC.md 8-92).

/// The three figures of this process one sample carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OwnReading {
    pub(crate) cpu_permille: u64,
    pub(crate) private_bytes: u64,
    pub(crate) working_set_bytes: u64,
}

/// The previous reading's CPU time and wall clock, which the next
/// reading's CPU share is measured against.
pub(crate) struct OwnProcess;

impl OwnProcess {
    pub(crate) fn new() -> Self {
        Self
    }

    /// Reads this process once. The first reading has nothing to compare
    /// with, so its CPU reads 0; a figure the platform refuses reads 0.
    pub(crate) fn read(&mut self) -> OwnReading {
        OwnReading::default()
    }
}

#[cfg(test)]
#[path = "own_process/tests.rs"]
mod tests;
