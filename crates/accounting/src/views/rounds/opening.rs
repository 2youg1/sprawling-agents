// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a session opened: its `run_started` line as the rounds answer it,
//! with the names its session froze read back from the content store
//! (`crates/wire/Spec.lean` §8-79, D17).

use std::path::Path;

use wire::{EventKind, EventRecord};

/// How the session opened, from the first `run_started` in the window,
/// with the names its session froze read back from `city_root`'s store.
#[must_use]
pub(super) fn opening(records: &[EventRecord], city_root: &Path) -> Option<wire::Opening> {
    records
        .iter()
        .find(|record| record.kind() == EventKind::RunStarted)
        .map(|record| {
            // A `run_started` this build cannot read still opened a
            // session; the page shows it with no words rather than
            // dropping the session from the account.
            let started = record
                .data()
                .read::<kernel::event::record::RunStarted>()
                .unwrap_or_default();
            wire::Opening {
                task: started.task,
                goal: started.goal,
                at: record.t(),
                dispatched_by: started.dispatched_by,
                policy: started.policy,
                effort: started.effort,
                names: started
                    .naming
                    .and_then(|version| frozen_names(city_root, &version)),
            }
        })
}

/// The names a session froze under `version`, as the content store holds
/// them (`crates/wire/Spec.lean` D17). A store that will not open, a
/// version it no longer holds and bytes that are not a frozen naming
/// answer no names: the page then falls back to the address and the
/// role's name, never to the names as they are today.
fn frozen_names(city_root: &Path, version: &kernel::B3Hash) -> Option<wire::FrozenNames> {
    let store = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).ok()?;
    let naming = city::Naming::from_bytes(&store.get(version).ok()?).ok()?;
    Some(wire::FrozenNames {
        mayor: naming.mayor().map(str::to_owned),
    })
}
