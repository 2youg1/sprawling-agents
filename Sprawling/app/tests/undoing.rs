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

use kernel::{IdemKey, RunId, Seq};

use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

fn a_city(dir: &std::path::Path) -> accounting::worker::RunWorker {
    form(
        dir,
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    accounting::worker::RunWorker::new(
        dir,
        runtime::diagnostics::Diagnostics::off(),
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
}

fn key(of: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, of)
}

#[test]
fn setting_autonomy_back_leaves_the_city_where_it_started() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = a_city(dir.path());
    let resident = kernel::ResidentId::new("lab/room1").unwrap();

    let delegated = wire::Command::SetAutonomy {
        scope: wire::HaltScope::City,
        autonomy: kernel::Autonomy::Delegate(resident),
        idem: key(b"delegate"),
    };
    let back = wire::Command::SetAutonomy {
        scope: wire::HaltScope::City,
        autonomy: kernel::Autonomy::Owner,
        idem: key(b"back"),
    };
    let again = wire::Command::SetAutonomy {
        scope: wire::HaltScope::City,
        autonomy: kernel::Autonomy::Owner,
        idem: key(b"back-again"),
    };

    worker.handle(delegated).unwrap();
    worker.handle(back).unwrap();
    let once = accounting::views::ask(dir.path(), &wire::Query::Governance).unwrap();
    worker.handle(again).unwrap();
    let twice = accounting::views::ask(dir.path(), &wire::Query::Governance).unwrap();

    assert_eq!(
        once, twice,
        "a reversal that arrived twice is still one reversal"
    );
}

#[test]
fn choosing_a_model_needs_the_endpoint_the_earlier_choice_named() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = a_city(dir.path());

    let err = worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("nowhere").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            input: None,
            idem: key(b"choose"),
        })
        .unwrap_err();
    assert!(
        err.subject().contains("is not attached"),
        "the refusal names the endpoint: {err}"
    );
}
