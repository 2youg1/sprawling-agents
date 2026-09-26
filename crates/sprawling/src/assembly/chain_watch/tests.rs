// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Address;

use super::*;
use crate::assembly::init_city;

/// Changes one digit of the first line's timestamp in place, so the
/// line still parses and only the chain can tell.
fn break_the_first_line(city_root: &Path) {
    let first = memory::ledger_segments_at(&ledger_dir(city_root))
        .unwrap()
        .remove(0);
    let mut bytes = std::fs::read(&first).unwrap();
    let at = bytes.windows(4).position(|held| held == b"\"t\":").unwrap() + 4;
    bytes[at] = if bytes[at] == b'1' { b'2' } else { b'1' };
    std::fs::write(&first, bytes).unwrap();
}

#[test]
fn a_chain_broken_under_a_served_city_refuses_the_next_command_with_the_audits_reason() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        Diagnostics::off(),
    )
    .unwrap();
    break_the_first_line(dir.path());
    let memory::ChainAudit::Broken(reason) = memory::audit_chain(&ledger_dir(dir.path())).unwrap()
    else {
        panic!("the tampered ledger still audits whole");
    };

    worker
        .audit_chain_in_background(Diagnostics::off())
        .unwrap()
        .join()
        .unwrap();
    let refused = worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, Seq::FIRST, b"create"),
        })
        .err();

    assert_eq!(refused, Some(reason));
}
