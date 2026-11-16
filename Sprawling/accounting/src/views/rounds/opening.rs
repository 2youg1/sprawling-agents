// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use wire::{EventKind, EventRecord};

#[must_use]
pub(super) fn opening(records: &[EventRecord], city_root: &Path) -> Option<wire::Opening> {
    records
        .iter()
        .find(|record| record.kind() == EventKind::RunStarted)
        .map(|record| {

            let started = record
                .data()
                .read::<kernel::event::record::RunStarted>()
                .unwrap_or_default();
            wire::Opening {
                task: started.task,
                goal: started.goal,
                at: record.t(),
                dispatched_by: started.dispatched_by,
                parent: started.parent,
                policy: started.policy,
                effort: started.effort,
                generation: started.generation,
                names: started
                    .naming
                    .and_then(|version| frozen_names(city_root, &version)),
            }
        })
}

fn frozen_names(city_root: &Path, version: &kernel::B3Hash) -> Option<wire::FrozenNames> {
    let store = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).ok()?;
    let naming = city::Naming::from_bytes(&store.get(version).ok()?).ok()?;
    Some(wire::FrozenNames {
        mayor: naming.mayor().map(str::to_owned),
    })
}
