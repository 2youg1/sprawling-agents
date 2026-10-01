// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxCode, Seq};

use super::*;
use crate::assembly::{hands, init_city};
use accounting::worker::RunWorker;

/// Changes one digit of the first line's timestamp in place, so the
/// line still parses and only the chain can tell.
fn break_the_first_line(city_root: &Path) {
    let first = storage::ledger_segments_at(&kernel::layout::CityLayout::new(city_root).ledger())
        .unwrap()
        .remove(0);
    let mut bytes = std::fs::read(&first).unwrap();
    let at = bytes
        .windows(4)
        .position(|held| held == b"\"t\":")
        .and_then(|key| key.checked_add(4))
        .unwrap();
    bytes[at] = if bytes[at] == b'1' { b'2' } else { b'1' };
    std::fs::write(&first, bytes).unwrap();
}

#[test]
fn a_chain_broken_under_a_served_city_refuses_the_next_command_with_the_audits_reason() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        Diagnostics::off(),
        hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    break_the_first_line(dir.path());
    let storage::ChainAudit::Broken(reason) =
        storage::audit_chain(&kernel::layout::CityLayout::new(dir.path()).ledger()).unwrap()
    else {
        panic!("the tampered ledger still audits whole");
    };

    audit_in_background(
        worker.chain_under_audit(storage::ChainHalt::awaiting_proof()),
        Diagnostics::off(),
        monotonic_now(),
    )
    .unwrap()
    .join()
    .unwrap();
    let refused = worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, Seq::FIRST, b"create"),
        })
        .err();

    assert_eq!(refused, Some(reason));
}

/// An audit that did not finish proved nothing whole, and a view that
/// resumed from a snapshot has only this audit looking at the lines
/// before it.
#[test]
fn an_audit_that_cannot_read_the_ledger_trips_the_halt() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-ledger-here");
    let reason = storage::audit_chain(&missing).unwrap_err().into_ax();
    let halt = storage::ChainHalt::awaiting_proof();

    report_proof(
        ChainUnderAudit {
            halt: halt.clone(),
            ledger_dir: missing.clone(),
            at: Seq::FIRST,
            records: storage::ProofRecords::read_only(&dir.path().join("records")),
        },
        Diagnostics::off(),
        monotonic_now(),
    );

    assert_eq!(halt.reason(), Some(reason));
}

/// A served worker answers a command that arrives before the proof of its
/// history with `E_HISTORY_UNPROVEN` and writes nothing, and takes the
/// same command once the proof is whole (sprawling-SPEC.md 8-90).
#[test]
fn a_served_worker_takes_commands_once_its_history_is_proved() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        Diagnostics::off(),
        hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    let create = |name: &str| wire::Command::CreateBuilding {
        addr: Address::parse(name).unwrap(),
        template: wire::TemplateName::parse("minimal").unwrap(),
        idem: kernel::IdemKey::derive(&RunId::CITY, Seq::FIRST, name.as_bytes()),
    };
    let watch = worker.chain_under_audit(storage::ChainHalt::awaiting_proof());

    let before = worker
        .handle(create("lab"))
        .map_err(|refused| *refused.code());
    audit_in_background(watch, Diagnostics::off(), monotonic_now())
        .unwrap()
        .join()
        .unwrap();
    let after = worker.handle(create("lab")).map(|_| ());

    assert_eq!(
        (before.err(), after),
        (Some(AxCode::HistoryUnproven), Ok(()))
    );
}
