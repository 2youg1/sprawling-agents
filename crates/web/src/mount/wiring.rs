// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The wiring: every signal one page holds.

use dioxus::prelude::*;

use channels::EventRecord;

use crate::app::Snapshot;
use crate::route::View;

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "the only caller is the browser's socket")
)]
pub(crate) struct Wiring {
    pub(crate) snapshot: Signal<Snapshot>,
    pub(crate) endpoints: Signal<Option<channels::EndpointsAnswer>>,
    pub(crate) city: Signal<Option<channels::CityAnswer>>,
    pub(crate) cost: Signal<Option<channels::CostAnswer>>,
    pub(crate) building: Signal<Option<channels::BuildingAnswer>>,
    pub(crate) discards: Signal<Option<channels::DiscardAnswer>>,
    pub(crate) inbox: Signal<Option<channels::InboxAnswer>>,
    pub(crate) hits: Signal<Option<channels::ArchiveAnswer>>,
    pub(crate) filed: Signal<Option<channels::RegistryAnswer>>,
    pub(crate) vitals: Signal<Option<channels::MetricsAnswer>>,
    pub(crate) changes: Signal<Option<channels::ChangesAnswer>>,
    pub(crate) records: Signal<Vec<EventRecord>>,
    pub(crate) live: Signal<bool>,
    /// Which page is showing, so the run a person just asked for can be
    /// opened when it starts.
    pub(crate) view: Signal<View>,
    /// The room this client last dispatched to, as `building/name`.
    /// Cleared by the run it was waiting for; see [`started_here`].
    pub(crate) expecting: Signal<Option<String>>,
    /// The last thing the city refused. Beside the snapshot rather than
    /// inside it: a refusal is not something that happened to the city,
    /// it is the answer to something one person asked, and the snapshot
    /// holds only what the ledger says.
    pub(crate) refused: Signal<Option<crate::alert::Refused>>,
    /// What language the words this wiring produces are said in. A
    /// signal rather than a value: these closures speak long after the
    /// page mounted.
    pub(crate) lang: Signal<crate::lang::Lang>,
}
