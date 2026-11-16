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

use kernel::{Address, AxCode, IdemKey, RunId, Seq};

use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

#[test]
fn a_halted_city_names_the_halt_and_not_the_configuration() {
    let dir = tempfile::tempdir().unwrap();
    form(
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
            addr: Address::parse("acme").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step1"),
        })
        .unwrap();

    let refused2 = worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("acme").unwrap(),
            task: "say something".to_owned(),
            goal: "an answer".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step2"),
            session: Some(kernel::SessionName::parse("one").unwrap()),
            effort: None,
            model: None,
        })
        .unwrap_err();
    assert_eq!(*refused2.code(), AxCode::ModelUnchosen);

    assert!(!refused2.recovery().is_empty());

    worker
        .handle(wire::Command::Halt {
            scope: wire::HaltScope::City,
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step3"),
        })
        .unwrap();

    let refused4 = worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("acme").unwrap(),
            task: "say something".to_owned(),
            goal: "an answer".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step4"),
            session: Some(kernel::SessionName::parse("two").unwrap()),
            effort: None,
            model: None,
        })
        .unwrap_err();
    assert_eq!(*refused4.code(), AxCode::GateDenied);

    assert!(!refused4.recovery().is_empty());
}

#[test]
fn every_spelling_of_one_endpoint_is_registered_as_one_url() {
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

    attach(&mut worker, "relay-0", "http://127.0.0.1:47199/v1").unwrap();
    attach(&mut worker, "relay-1", "http://127.0.0.1:47199/v1/").unwrap();
    attach(&mut worker, "relay-2", "http://127.0.0.1:47199/v1/messages").unwrap();
    attach(&mut worker, "relay-3", "http://127.0.0.1:47199").unwrap();
    attach(&mut worker, "relay-4", "127.0.0.1:47199").unwrap();

    let written = stated(
        &raised.ledger_dir,
        kernel::EventKind::EndpointAttached,
        "base_url",
    );

    assert_eq!(written.len(), 5);
    assert!(
        written.windows(2).all(|pair| pair[0] == pair[1]),
        "one endpoint was written down as {written:?}"
    );
}

#[test]
fn a_model_no_catalogue_prices_is_registered_with_a_ceiling() {
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

    attach(&mut worker, "relay", "http://127.0.0.1:47199/v1").unwrap();

    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("relay").unwrap(),
            model: "opus-nine".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            input: None,
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"select"),
        })
        .unwrap();

    let ceilings = stated(
        &raised.ledger_dir,
        kernel::EventKind::ModelSelected,
        "max_output_tokens",
    );
    assert!(
        ceilings
            .first()
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|tokens| tokens > 0),
        "a model nobody priced was registered with {ceilings:?}"
    );

    let rungs = stated(
        &raised.ledger_dir,
        kernel::EventKind::ModelSelected,
        "ceiling_from",
    );
    assert_eq!(
        rungs.first().and_then(serde_json::Value::as_str),
        Some(gateway::FigureSource::Policy.as_str())
    );
}

fn attach(
    worker: &mut accounting::worker::RunWorker,
    name: &str,
    base_url: &str,
) -> Result<(), kernel::AxError> {
    worker.handle(wire::Command::AttachEndpoint {
        name: wire::ProviderName::parse(name)?,
        base_url: base_url.to_owned(),
        dialect: kernel::DialectKind::Anthropic,
        secret: None,
        auth_header: None,
        admit: vec!["opus-nine".to_owned()],
        named: Vec::new(),
        tuning: wire::EndpointTuning::default(),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, name.as_bytes()),
    })
}

fn stated(
    ledger: &std::path::Path,
    kind: kernel::EventKind,
    field: &str,
) -> Vec<serde_json::Value> {
    let verified = runtime::replay::verify_ledger_dir(ledger).unwrap();
    verified
        .raw_lines()
        .iter()
        .filter_map(|line| {
            let record = kernel::EventRecord::parse_line(line).unwrap();
            if record.kind() != kind {
                return None;
            }
            record.data().as_map().get(field).cloned()
        })
        .collect()
}
