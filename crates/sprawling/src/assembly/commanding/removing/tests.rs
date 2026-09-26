// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Removing a building through the worker's own door: the refusal lends
//! a room's queue the way a dispatch does, and the removal is read back
//! from the Ledger line it wrote.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]

use crate::assembly::fixture::*;
use crate::assembly::*;

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

fn create(worker: &mut RunWorker, name: &str) {
    worker
        .handle(channels::Command::CreateBuilding {
            addr: addr(name),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, name.as_bytes()),
        })
        .unwrap();
}

#[test]
fn a_building_with_a_run_going_stays_and_an_idle_one_leaves_with_its_record() {
    let dir = tempfile::tempdir().unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    create(&mut worker, "lab");
    create(&mut worker, "mill");
    let running = kernel::RunId::from_bytes([9u8; 16]);
    let _lent = worker.rooms.lend(&addr("lab/refactor"), running);

    let busy = worker
        .remove_building(&addr("lab"))
        .map_err(|err| *err.code());
    let idle = worker
        .remove_building(&addr("mill"))
        .map_err(|err| *err.code());

    let verified = runtime::replay::verify_ledger_dir(&ledger_dir(dir.path())).unwrap();
    let recorded: Vec<(String, String)> = verified
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter(|line| line["kind"] == "building_removed")
        .map(|line| {
            (
                line["data"]["addr"].as_str().unwrap().to_owned(),
                line["data"]["kept"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        (busy, idle, city::buildings(dir.path()).unwrap(), recorded),
        (
            Err(kernel::AxCode::Busy),
            Ok(()),
            vec![addr("lab")],
            vec![("mill".to_owned(), ".sprawling/removed/mill".to_owned())]
        )
    );
}
