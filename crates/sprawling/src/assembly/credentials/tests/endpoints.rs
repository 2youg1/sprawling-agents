// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which models a city may reach, and what an adapter redeems at the
//! wire.

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

#[test]
fn a_model_the_endpoint_never_listed_cannot_be_chosen() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small"], Vec::new());
    let Err(err) = worker_with_provider(dir.path(), &base_url, "m-invented") else {
        panic!("a model the endpoint never listed cannot be chosen");
    };
    assert_eq!(*err.code(), AxCode::ConfigInvalid);
    assert!(err.subject().contains("m-invented"));
}

/// Most compatible endpoints serve no model list, so the ids the
/// person declared are the only ids there will ever be. Attaching on
/// them is what the four reference harnesses do; refusing would keep a
/// working provider out of the city over an interface it never
/// promised.
#[test]
fn an_endpoint_with_no_model_list_attaches_on_the_ids_the_person_named() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&[], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("declared").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["m-1".to_owned()],
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    let held = worker.book.endpoints().next().unwrap().clone();
    assert_eq!(held.models, vec!["m-1".to_owned()]);
    assert!(
        !held.probed,
        "this list is the person's word, and the book says so"
    );
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("declared").unwrap(),
            model: "m-1".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 32_768,
            max_output_tokens: 4_096,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
}

#[test]
fn an_endpoint_with_neither_a_model_list_nor_a_declared_id_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&[], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let err = worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("silent").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
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
    init_city(dir.path()).unwrap();
    // Nothing registered: the refusal has to name the act that fixes
    // it, because a person who has not attached a provider yet is
    // exactly the person who does not know that is the missing step.
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let err = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "anything".to_owned(),
            goal: "anything".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
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
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-key"], vec![completion("done", None)]);
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::PutSecret {
            realm: "proxy".to_owned(),
            name: "key".to_owned(),
            value: kernel::Sealed::new(Box::new("sk-proxy-credential".to_owned())),
        })
        .unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("proxied").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: Some("secret:proxy/key".to_owned()),
            auth_header: None,
            admit: Vec::new(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("proxied").unwrap(),
            model: "m-key".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 32_768,
            max_output_tokens: 4_096,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "say done".to_owned(),
            goal: "auth on the wire".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    let chat = provider
        .exchanges()
        .into_iter()
        .find(|head| head.starts_with("POST"))
        .expect("the dispatch called the model");
    // Before the credential-aware route existed, a loopback endpoint
    // with a secret went through the local adapter and this header
    // was silently absent - the probe authenticated, the calls never.
    assert!(
        chat.to_ascii_lowercase()
            .contains("authorization: bearer sk-proxy-credential"),
        "the chat call must carry the credential; head was:\n{chat}"
    );
}

/// The content store is opened once, when the adapter is built, rather
/// than once per picture (sprawling-SPEC.md 8-50). What that moves is
/// where the failure lands: a city whose store cannot be opened says so
/// while the adapter is being assembled, instead of half way through a
/// conversation the model has already started.
#[test]
fn a_store_that_will_not_open_refuses_the_adapter_rather_than_the_first_picture() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    assert!(
        worker.redemption().is_ok(),
        "a city that has a content store redeems from it"
    );

    // Where the store belongs there is now a file, so opening it is a
    // failure this city cannot recover from by itself.
    let cas = dir.path().join(".sprawling").join("cas");
    std::fs::remove_dir_all(&cas).unwrap();
    std::fs::write(&cas, b"not a directory").unwrap();
    let err = match worker.redemption() {
        Err(err) => err,
        Ok(_) => panic!("the store was opened lazily again: nothing failed here"),
    };
    assert_eq!(err.action(), "read a picture");
}
