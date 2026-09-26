// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]

use crate::assembly::*;
use kernel::degradation::VolumeSpace;

const GIB: u64 = 1024 * 1024 * 1024;

/// One GiB free on a 1000 GiB volume, whose floor is 4 GiB.
fn close_to_full(_city: &Path) -> Option<VolumeSpace> {
    Some(VolumeSpace {
        free_bytes: GIB,
        total_bytes: 1000 * GIB,
    })
}

#[test]
fn a_volume_below_its_floor_refuses_a_dispatch_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker.read_volume_with(close_to_full);

    let refused = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/east").unwrap(),
            task: "fire the east kiln".to_owned(),
            goal: String::new(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .map(drop);

    let told = refused
        .as_ref()
        .map_err(|e| (*e.code(), e.recovery().to_owned()));
    assert_eq!(
        told,
        Err((
            AxCode::BackpressureShed,
            format!("free at least {} bytes on the city's volume", 3 * GIB),
        ))
    );
    assert!(
        !dir.path().join("lab").join("east").exists(),
        "nothing was written for the refused work"
    );
}
