// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use sysinfo::{MemoryRefreshKind, System};

use accounting::worker::Memory;

thread_local! {

    static SYSTEM: std::cell::Cell<Option<System>> = const { std::cell::Cell::new(None) };
}

#[must_use]
pub fn read() -> Memory {
    SYSTEM.with(|kept| {
        let mut system = kept.take().unwrap_or_else(System::new);
        let memory = refreshed(&mut system);
        kept.set(Some(system));
        memory
    })
}

pub(crate) fn refreshed(system: &mut System) -> Memory {
    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    Memory {
        physical: system.total_memory(),
        available: system.available_memory(),
    }
}
