// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_model_the_endpoint_never_listed_cannot_be_chosen() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small"], Vec::new());
    let Err(err) = worker_with_provider(dir.path(), &base_url, "m-invented") else {
        panic!("a model the endpoint never listed cannot be chosen");
    };
    assert_eq!(*err.code(), AxCode::ConfigInvalid);
    assert!(err.subject().contains("m-invented"));
}

#[test]
fn an_endpoint_with_no_model_list_attaches_on_the_ids_the_person_named() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&[], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("declared").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["m-1".to_owned()],
            named: Vec::new(),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    let held = worker.credentials.book.endpoints().next().unwrap().clone();
    assert_eq!(
        held.models
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<String>>(),
        vec!["m-1".to_owned()]
    );
    assert!(
        !held.probed,
        "this list is the person's word, and the book says so"
    );
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("declared").unwrap(),
            model: "m-1".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
}

#[test]
fn an_endpoint_with_neither_a_model_list_nor_a_declared_id_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&[], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let err = worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("silent").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            named: Vec::new(),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap_err();
    assert!(
        err.recovery().contains("model"),
        "a city with no model id to call must ask for one: {}",
        err.recovery()
    );
}

#[test]
fn a_dispatch_without_a_provider_fails_saying_what_to_configure() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();

    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let err = worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "anything".to_owned(),
            goal: "anything".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap_err();
    assert!(
        err.recovery().contains("settings page"),
        "got: {}",
        err.recovery()
    );
}

#[test]
fn a_loopback_endpoint_with_a_credential_sends_it_on_every_call() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-key"], vec![completion("done", None)]);
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::PutSecret {
            realm: "proxy".to_owned(),
            name: "key".to_owned(),
            value: kernel::Sealed::new(Box::new("sk-proxy-credential".to_owned())),
        })
        .unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("proxied").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: Some("secret:proxy/key".to_owned()),
            auth_header: None,
            admit: Vec::new(),
            named: Vec::new(),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("proxied").unwrap(),
            model: "m-key".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "say done".to_owned(),
            goal: "auth on the wire".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let chat = provider
        .exchanges()
        .into_iter()
        .find(|head| head.starts_with("POST"))
        .expect("the dispatch called the model");

    assert!(
        chat.to_ascii_lowercase()
            .contains("authorization: bearer sk-proxy-credential"),
        "the chat call must carry the credential; head was:\n{chat}"
    );
}

#[test]
fn a_store_that_will_not_open_refuses_the_adapter_rather_than_the_first_picture() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    assert!(
        worker.redemption().is_ok(),
        "a city that has a content store redeems from it"
    );

    let cas = dir.path().join(".sprawling").join("cas");
    std::fs::remove_dir_all(&cas).unwrap();
    std::fs::write(&cas, b"not a directory").unwrap();
    let err = match worker.redemption() {
        Err(err) => err,
        Ok(_) => panic!("the store was opened lazily again: nothing failed here"),
    };
    assert_eq!(err.action(), "read a picture");
}

#[test]
fn a_probe_that_reaches_nothing_records_where_it_stopped_rather_than_refusing() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::ProbeEndpoint {
            name: wire::ProviderName::parse("nowhere").unwrap(),

            base_url: "https://gateway.invalid/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"probe"),
        })
        .expect("a probe that reaches nothing still answers");
    let written = ledger_text(&report.ledger_dir);
    assert!(written.contains("endpoint_probed"), "{written}");
    assert!(written.contains("gateway.invalid"), "{written}");
    assert!(
        written.contains("\"failed\""),
        "the refusal travels beside the reading: {written}"
    );
}

#[test]
fn a_probe_carries_the_facts_each_model_row_stated() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small", "m-large"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::ProbeEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"probe"),
        })
        .unwrap();
    let written = ledger_text(&report.ledger_dir);
    assert!(written.contains("\"facts\""), "{written}");
    assert!(written.contains("\"reach\""), "{written}");
    assert!(
        !written.contains("\"failed\""),
        "a list that came back is not a failure: {written}"
    );
}

pub(super) fn ledger_text(ledger_dir: &std::path::Path) -> String {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n")
}

pub(super) fn selected_lines(ledger_dir: &std::path::Path) -> Vec<serde_json::Value> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| kernel::EventRecord::parse_line(line).unwrap())
        .filter(|record| record.kind() == kernel::EventKind::ModelSelected)
        .map(|record| {
            let data = record.data().as_map();
            let mut picked = serde_json::Map::new();
            for key in [
                "context_tokens",
                "window_from",
                "max_output_tokens",
                "ceiling_from",
            ] {
                picked.insert(
                    key.to_owned(),
                    data.get(key).cloned().unwrap_or(serde_json::Value::Null),
                );
            }
            serde_json::Value::Object(picked)
        })
        .collect()
}
