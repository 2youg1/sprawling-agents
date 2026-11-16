// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use super::endpoints::{ledger_text, selected_lines};
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_line_a_run_writes_reaches_the_book_the_worker_holds() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let attached = kernel::event::record::EndpointAttached {
        name: "by-a-run".to_owned(),
        base_url: "http://127.0.0.1:9".to_owned(),
        dialect: kernel::DialectKind::OpenAi,
        auth: None,
        auth_header: None,
        models: vec!["m-1".to_owned()],
        connection_kind: None,
        probed: false,
        tuning: None,
        facts_blob: None,
    };
    worker
        .record_for(
            kernel::RunId::from_bytes([7; 16]),
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: Address::parse("lab/room1").unwrap(),
                kind: EventKind::EndpointAttached,
                data: Payload::of(&attached).unwrap(),
            },
        )
        .unwrap();
    let names = |book: &gateway::EndpointBook| {
        book.endpoints()
            .map(|held| held.name.clone())
            .collect::<Vec<String>>()
    };
    let rebuilt = Standing::fold(&report.ledger_dir).unwrap().book;
    assert_eq!(names(&rebuilt), vec!["by-a-run".to_owned()]);
    assert_eq!(
        names(&worker.credentials.book),
        names(&rebuilt),
        "the live book and the restart's book are one fold"
    );
}

#[test]
fn a_pick_that_states_nothing_registers_what_the_providers_list_states() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-stated"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            named: Vec::new(),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-stated".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    let selected = selected_lines(&report.ledger_dir);
    assert_eq!(
        selected,
        vec![serde_json::json!({
            "context_tokens": LISTED_WINDOW,
            "window_from": "upstream",
            "max_output_tokens": LISTED_CEILING,
            "ceiling_from": "upstream",
        })]
    );
}

#[test]
fn a_re_attach_whose_list_cannot_be_read_keeps_the_facts_already_registered() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-stated", "m-plain"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let attach = |worker: &mut RunWorker, admit: &[&str], idem: &[u8]| {
        worker
            .handle(wire::Command::AttachEndpoint {
                name: wire::ProviderName::parse("house").unwrap(),
                base_url: base_url.clone(),
                dialect: kernel::DialectKind::OpenAi,
                secret: None,
                auth_header: None,
                admit: admit.iter().map(|id| (*id).to_owned()).collect(),
                named: Vec::new(),
                tuning: wire::EndpointTuning::default(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
            })
            .unwrap();
        let held = worker.credentials.book.endpoints().next().unwrap().clone();
        (held.models, held.probed)
    };
    let (read, probed) = attach(&mut worker, &[], b"listed");
    assert!(probed);
    let stated = read
        .iter()
        .find(|row| row.id == "m-stated")
        .unwrap()
        .clone();
    assert_eq!(stated.context_tokens, Some(LISTED_WINDOW));
    provider.stop_listing();
    assert_eq!(
        attach(&mut worker, &["m-stated"], b"unread, nothing new"),
        (vec![stated.clone()], true),
        "a failed probe reset what the earlier one read"
    );
    assert_eq!(
        attach(&mut worker, &["m-stated", "m-new"], b"unread, one new"),
        (
            vec![
                stated,
                gateway::ModelFacts {
                    id: "m-new".to_owned(),
                    listing: kernel::event::record::Listing::Named,
                    ..gateway::ModelFacts::default()
                },
            ],
            false,
        )
    );
}

fn attach_relay(
    worker: &mut RunWorker,
    base_url: &str,
    (ticked, typed): (&[&str], &[&str]),
    idem: &[u8],
) -> Result<(), AxError> {
    let owned = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect();
    worker.handle(wire::Command::AttachEndpoint {
        name: wire::ProviderName::parse("relay").unwrap(),
        base_url: base_url.to_owned(),
        dialect: kernel::DialectKind::OpenAi,
        secret: None,
        auth_header: None,
        admit: owned(ticked),
        named: owned(typed),
        tuning: wire::EndpointTuning::default(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
    })
}

fn relay_rows(worker: &RunWorker) -> (Vec<(String, kernel::event::record::Listing)>, bool) {
    let held = worker
        .credentials
        .book
        .endpoints()
        .find(|endpoint| endpoint.name == "relay")
        .unwrap();
    (
        held.models
            .iter()
            .map(|row| (row.id.clone(), row.listing))
            .collect(),
        held.probed,
    )
}

#[test]
fn a_typed_id_the_list_does_not_carry_is_registered_as_the_persons_word() {
    use kernel::event::record::Listing;
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["relay-chat-1", "deepseek-chat"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    attach_relay(
        &mut worker,
        &base_url,
        (&["relay-chat-1"], &["relay-hidden-1"]),
        b"attach",
    )
    .unwrap();
    assert_eq!(
        relay_rows(&worker),
        (
            vec![
                ("relay-chat-1".to_owned(), Listing::Listed),
                ("relay-hidden-1".to_owned(), Listing::Named),
            ],
            false,
        )
    );
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("relay").unwrap(),
            model: "relay-hidden-1".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
}

fn untouched(worker: &RunWorker, ledger_dir: &std::path::Path) -> (serde_json::Value, String) {
    (
        serde_json::to_value(&worker.credentials.book).unwrap(),
        ledger_text(ledger_dir),
    )
}

#[test]
fn a_ticked_list_that_matches_nothing_served_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["relay-chat-1", "deepseek-chat"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    attach_relay(&mut worker, &base_url, (&["relay-chat-1"], &[]), b"first").unwrap();
    let before = untouched(&worker, &report.ledger_dir);
    let err = attach_relay(
        &mut worker,
        &base_url,
        (&["relay-hidden-1"], &[]),
        b"second",
    )
    .expect_err("a ticked list that matches nothing is refused");

    let mut offered = err.nearby().to_vec();
    offered.sort();
    assert_eq!(
        (
            *err.code(),
            err.subject().contains("relay-hidden-1"),
            offered
        ),
        (
            AxCode::ConfigInvalid,
            true,
            vec!["deepseek-chat".to_owned(), "relay-chat-1".to_owned()],
        )
    );
    assert_eq!(untouched(&worker, &report.ledger_dir), before);
}

#[test]
fn a_re_attach_that_drops_the_model_a_role_points_at_is_refused_naming_the_role() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["relay-chat-1", "deepseek-chat"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    attach_relay(
        &mut worker,
        &base_url,
        (&["relay-chat-1", "deepseek-chat"], &[]),
        b"both",
    )
    .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("relay").unwrap(),
            model: "deepseek-chat".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    let before = untouched(&worker, &report.ledger_dir);
    let err = attach_relay(&mut worker, &base_url, (&["relay-chat-1"], &[]), b"one")
        .expect_err("dropping the model main points at is refused");
    assert_eq!(
        (*err.code(), err.subject()),
        (
            AxCode::ConfigInvalid,
            "deepseek-chat is main; choose another main first"
        )
    );
    assert_eq!(untouched(&worker, &report.ledger_dir), before);
}
