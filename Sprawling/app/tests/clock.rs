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

use std::sync::Arc;

use kernel::{Address, AxError, EventRecord, IdemKey, RunId, Seq, TimeMs};

use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

const STILL: u64 = 1_000_000;

struct Stopped;

impl accounting::Clock for Stopped {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(STILL))
    }
}

#[test]
fn a_worker_stamps_its_lines_with_the_clock_it_was_handed() {
    let dir = tempfile::tempdir().unwrap();
    let raised = form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    let before = runtime::replay::verify_ledger_dir(&raised.ledger_dir)
        .unwrap()
        .raw_lines()
        .len();
    let mut worker = accounting::worker::RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
    .with_clock(Arc::new(Stopped));
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"create"),
        })
        .unwrap();

    let stamped: Vec<u64> = runtime::replay::verify_ledger_dir(&raised.ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .skip(before)
        .map(|line| EventRecord::parse_line(line).unwrap().t().value())
        .collect();
    assert!(!stamped.is_empty(), "creating a building wrote nothing");
    assert_eq!(stamped, vec![STILL; stamped.len()]);
}
