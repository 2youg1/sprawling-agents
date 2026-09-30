// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use kernel::{Address, EventKind, EventRecord, Payload, RunId};

mod folding;
mod released;
mod released_ledger;
mod roadmaps;

/// A city as genesis leaves it, as far as a view test reads it.
pub struct Founded {
    /// Where the city's ledger is.
    pub ledger_dir: std::path::PathBuf,
}

/// Forms a city the way genesis does, without a worker: the ledger's
/// first two lines, the ignore rules, a norms file, the store, and City
/// Hall laid out and recorded. Stands in for `genesis` until the worker
/// moves into this crate (accounting-SPEC.md 12-16).
pub fn founded(city_root: &std::path::Path) -> Founded {
    use kernel::Ledger;
    let layout = kernel::layout::CityLayout::new(city_root);
    let ledger_dir = layout.ledger();
    std::fs::create_dir_all(&ledger_dir).unwrap();
    let t = kernel::TimeMs::new(1);
    let (mut ledger, _opened) = storage::JsonlLedger::open(&ledger_dir, t).unwrap();
    let mut write = |who: &str, addr: Option<Address>, kind: EventKind, data: Payload| {
        ledger
            .append(kernel::EventDraft {
                run: RunId::CITY,
                t,
                who: who.to_owned(),
                addr,
                kind,
                data,
                ig: false,
            })
            .unwrap();
    };
    write(
        "city",
        layout.city_address(),
        EventKind::CityInitialized,
        Payload::of(&kernel::event::record::CityInitialized {}).unwrap(),
    );
    let clerk = kernel::ResidentId::new(kernel::consts_policy::HALL_CLERK).unwrap();
    write(
        "city",
        None,
        EventKind::AutonomyChanged,
        Payload::of(&kernel::event::record::AutonomyChanged {
            scope: kernel::event::Scope::City,
            autonomy: kernel::Autonomy::Delegate(clerk),
        })
        .unwrap(),
    );
    city::ignore_city_records(city_root).unwrap();
    std::fs::write(city_root.join(city::CITY_FILE), "# City\n").unwrap();
    storage::Cas::open(&layout.cas()).unwrap();
    let plan = city::CityPlan::new(None).unwrap();
    let (hall, template) = plan.hall();
    let building = city::create_building(city_root, hall, *template).unwrap();
    write(
        "owner",
        None,
        EventKind::BuildingCreated,
        city::building_created_payload(&building, *template).unwrap(),
    );
    city::lay_out_hall_identities(city_root).unwrap();
    Founded { ledger_dir }
}

/// Where a test line sits in the ledger and which run wrote it.
pub(super) struct Place {
    pub(super) seq: u64,
    pub(super) run: RunId,
}

/// One record for a view test, carrying a payload and an address.
pub(super) fn view_record(
    Place { seq, run }: Place,
    kind: EventKind,
    addr: &Address,
    data: serde_json::Map<String, serde_json::Value>,
) -> EventRecord {
    EventRecord::from_draft(
        kernel::EventDraft {
            run,
            t: kernel::TimeMs::new(1_000),
            who: "lab/room1".to_owned(),
            addr: Some(addr.clone()),
            kind,
            data: Payload::new(data).unwrap(),
            ig: false,
        },
        kernel::Seq::new(seq),
        kernel::B3Hash::digest(b"prev"),
    )
}
