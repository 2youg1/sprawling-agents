// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's memory, read from the platform through `sysinfo`, the
//! one place the city reads it (`crates/sprawling/spec/Monitor.lean` §8-94).

use sysinfo::{MemoryRefreshKind, System};

use accounting::worker::Memory;

thread_local! {
    /// The handle `read` refreshes, kept per thread so a reading pays the
    /// platform call and not a new handle, with no lock between threads.
    static SYSTEM: std::cell::Cell<Option<System>> = const { std::cell::Cell::new(None) };
}

/// Reads this machine's memory now, through this thread's kept handle.
#[must_use]
pub fn read() -> Memory {
    SYSTEM.with(|kept| {
        let mut system = kept.take().unwrap_or_else(System::new);
        let memory = refreshed(&mut system);
        kept.set(Some(system));
        memory
    })
}

/// Reads this machine's memory through a handle the caller keeps, as the
/// monitor's sampler does for its other counters. One platform call,
/// which refreshes the RAM figures and nothing else.
pub(crate) fn refreshed(system: &mut System) -> Memory {
    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    Memory {
        physical: system.total_memory(),
        available: system.available_memory(),
    }
}
