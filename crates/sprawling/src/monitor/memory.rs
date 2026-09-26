// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's memory, read from the platform through `sysinfo`, the
//! one place the city reads it (sprawling-SPEC.md 8-94).

use sysinfo::{MemoryRefreshKind, System};

/// Physical memory and how much of it the platform could hand out now,
/// in bytes; both zero where the platform does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Memory {
    pub(crate) physical: u64,
    pub(crate) available: u64,
}

/// Reads this machine's memory now. One platform call, which refreshes
/// the RAM figures and nothing else.
pub(crate) fn read() -> Memory {
    let mut system = System::new();
    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    Memory {
        physical: system.total_memory(),
        available: system.available_memory(),
    }
}
