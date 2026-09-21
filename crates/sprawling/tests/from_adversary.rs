// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The traces the adversary found, kept here so this repository remembers them.
//!
//! Written by `adversary/src/Sprawling/Regression.lean` and compared against it
//! byte for byte. Change the trace there; changing it here turns the adversary
//! red, which is exactly what should happen when the two disagree.
//!
//! The adversary drives the shipped binary over the wire. This enters by the
//! same door `channels::server` does, so the trace runs without a port.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{Address, AxCode, IdemKey, RunId, Seq};
use sprawling::assembly;

#[test]
fn a_halted_city_names_the_halt_and_not_the_configuration() {
    let dir = tempfile::tempdir().unwrap();
    assembly::init_city(dir.path()).unwrap();

    // The vault is the in-session one: a test that reached the platform
    // credential service would write to the machine running it.
    let mut worker = assembly::RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("acme").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step1"),
        })
        .unwrap();

    let refused2 = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("acme").unwrap(),
            task: "say something".to_owned(),
            goal: "an answer".to_owned(),
            mode: channels::ModeTag::parse("build").unwrap(),
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step2"),
            session: Some(kernel::SessionName::parse("one").unwrap()),
            effort: None,
        })
        .unwrap_err();
    assert_eq!(*refused2.code(), AxCode::ConfigInvalid);
    // A refusal is a promise in three parts; a code with no way forward
    // keeps only one of them.
    assert!(!refused2.recovery().is_empty());

    worker
        .handle(channels::Command::Halt {
            scope: channels::HaltScope::City,
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step3"),
        })
        .unwrap();

    let refused4 = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("acme").unwrap(),
            task: "say something".to_owned(),
            goal: "an answer".to_owned(),
            mode: channels::ModeTag::parse("build").unwrap(),
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"step4"),
            session: Some(kernel::SessionName::parse("two").unwrap()),
            effort: None,
        })
        .unwrap_err();
    assert_eq!(*refused4.code(), AxCode::GateDenied);
    // A refusal is a promise in three parts; a code with no way forward
    // keeps only one of them.
    assert!(!refused4.recovery().is_empty());
}

#[test]
fn every_spelling_of_one_endpoint_is_registered_as_one_url() {
    let dir = tempfile::tempdir().unwrap();
    let raised = assembly::init_city(dir.path()).unwrap();

    // The vault is the in-session one: a test that reached the platform
    // credential service would write to the machine running it.
    let mut worker = assembly::RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    // A provider prints one endpoint several ways and a person pastes
    // whichever one they were shown. Nothing listens on this address, so
    // every attachment below is one the person's own model id carried.
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
    // One registration per spelling, and the same URL in each: what is
    // asserted is the relation between the readings, so no normalisation
    // is recomputed here.
    assert_eq!(written.len(), 5);
    assert!(
        written.windows(2).all(|pair| pair[0] == pair[1]),
        "one endpoint was written down as {written:?}"
    );
}

#[test]
fn a_model_no_catalogue_prices_is_registered_with_a_ceiling() {
    let dir = tempfile::tempdir().unwrap();
    let raised = assembly::init_city(dir.path()).unwrap();

    // The vault is the in-session one: a test that reached the platform
    // credential service would write to the machine running it.
    let mut worker = assembly::RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    attach(&mut worker, "relay", "http://127.0.0.1:47199/v1").unwrap();
    // The box a person left empty. The Anthropic wire requires
    // `max_tokens` in every request, so a registration that kept the
    // ceiling unstated is a model this city cannot call at all.
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("relay").unwrap(),
            model: "opus-nine".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 0,
            max_output_tokens: None,
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
    // Which rung answered, read back in the ladder's own spelling: this
    // id is in no catalogue and under no preset host, so the policy
    // default is the rung that was left.
    let rungs = stated(
        &raised.ledger_dir,
        kernel::EventKind::ModelSelected,
        "ceiling_from",
    );
    assert_eq!(
        rungs.first().and_then(serde_json::Value::as_str),
        Some(gateway::CeilingSource::Policy.as_str())
    );
}

/// One endpoint attached under a name of its own, with the model id the
/// person named.
///
/// The id is named rather than left to the probe: most compatible
/// endpoints serve no model list, and this address serves nothing at
/// all, so an attachment that declared no id would be refused for an
/// interface the endpoint never promised.
fn attach(
    worker: &mut assembly::RunWorker,
    name: &str,
    base_url: &str,
) -> Result<(), kernel::AxError> {
    worker.handle(channels::Command::AttachEndpoint {
        name: channels::ProviderName::parse(name)?,
        base_url: base_url.to_owned(),
        dialect: kernel::DialectKind::Anthropic,
        secret: None,
        auth_header: None,
        admit: vec!["opus-nine".to_owned()],
        tuning: channels::EndpointTuning::default(),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, name.as_bytes()),
    })
}

/// What this city's history states under one kind of record and one
/// field, oldest first.
///
/// The ledger is the only reading these tests take: what a city
/// registered is what its history says it registered, and a field read
/// off a live structure would be a second account of the same fact.
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
