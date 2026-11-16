// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{Address, EventKind, EventRecord, IdemKey, RunId, Seq};

use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

const LAB: &str = "lab";

#[test]
fn a_command_reaches_the_ledger_through_the_door_the_wire_uses() {
    let dir = tempfile::tempdir().unwrap();
    let raised = form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();

    let mut worker = accounting::worker::RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();

    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse(LAB).unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"create"),
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let laid = verified.raw_lines().iter().any(|line| {
        let record = EventRecord::parse_line(line).unwrap();
        record.kind() == EventKind::BuildingCreated
            && record
                .data()
                .as_map()
                .get("addr")
                .and_then(serde_json::Value::as_str)
                == Some(LAB)
    });
    assert!(
        laid,
        "the command went through the door but never reached the history"
    );
}
