// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![cfg(test)]

use crate::worker::fixture::{fake_openai, worker_with_provider};

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(32))]
    #[test]
    fn account_submission_traces_preserve_explicit_authority(
        updates in proptest::collection::vec(
            (proptest::option::of(1usize..5), proptest::bool::ANY, proptest::bool::ANY),
            0..12,
        ),
    ) {
        use kernel::{IdemKey, RunId, Seq, ServerLabel};
        let dir = tempfile::tempdir().unwrap();
        let report = crate::worker::fixture::init_city(dir.path()).unwrap();
        let (base_url, provider) = fake_openai(&["m-1"], vec![]);
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
        let accounts = |count: usize| (0..count).map(|index| {
            kernel::event::record::ProviderAccount {
                id: ServerLabel::parse(&format!("account{index}")).unwrap(),
                reference: None,
                header: None,
            }
        }).collect::<Vec<_>>();
        let mut held = Some(accounts(1));
        let command = |incoming, secret, header, marker: &[u8]| wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url: base_url.clone(),
            dialect: kernel::DialectKind::OpenAi,
            secret,
            auth_header: header,
            admit: vec!["m-1".to_owned()],
            named: Vec::new(),
            tuning: wire::EndpointTuning { accounts: incoming, ..Default::default() },
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, marker),
        };
        worker.handle(command(held.clone(), None, None, b"migrate")).unwrap();
        for (index, (incoming, secret, header)) in updates.into_iter().enumerate() {
            let incoming = incoming.map(accounts);
            let before = serde_json::to_value(&worker.credentials.book).unwrap();
            let calls = provider.exchanges().len();
            let ledger = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
            let result = worker.handle(command(
                incoming.clone(),
                secret.then(|| "secret:fixture/c".to_owned()),
                header.then(|| "X-Legacy".to_owned()),
                format!("update-{index}").as_bytes(),
            ));
            if secret || header {
                proptest::prop_assert_eq!(*result.unwrap_err().code(), kernel::AxCode::ConfigInvalid);
                proptest::prop_assert_eq!(serde_json::to_value(&worker.credentials.book).unwrap(), before);
                proptest::prop_assert_eq!(provider.exchanges().len(), calls);
                proptest::prop_assert_eq!(
                    runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap().raw_lines().to_vec(),
                    ledger.raw_lines(),
                );
            } else {
                result.unwrap();
                held = incoming.or(held);
            }
            let actual = worker.credentials.book.endpoints().find(|endpoint| endpoint.name == "house")
                .unwrap().tuning.accounts.clone();
            proptest::prop_assert_eq!(&actual, &held);
            proptest::prop_assert!(actual.is_some());
        }
    }
}

#[test]
fn explicit_accounts_refuse_legacy_probe_fields_before_effects() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-1"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
    for id in ["a", "b"] {
        worker
            .handle(wire::Command::PutSecret {
                realm: "fixture".to_owned(),
                name: id.to_owned(),
                value: kernel::Sealed::new(Box::new(format!("fixture-{id}"))),
            })
            .unwrap();
    }
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url: base_url.clone(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["m-1".to_owned()],
            named: Vec::new(),
            tuning: wire::EndpointTuning {
                accounts: Some(
                    ["a", "b"]
                        .map(|id| kernel::event::record::ProviderAccount {
                            id: kernel::ServerLabel::parse(id).unwrap(),
                            reference: Some(kernel::SecretRef::new("fixture", id).unwrap()),
                            header: Some("X-Account".to_owned()),
                        })
                        .to_vec(),
                ),
                ..Default::default()
            },
            idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"migrate"),
        })
        .unwrap();
    worker
        .handle(wire::Command::ProbeEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url: base_url.clone(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(
                &kernel::RunId::CITY,
                kernel::Seq::FIRST,
                b"probe-migrated",
            ),
        })
        .unwrap();
    assert!(
        provider
            .exchanges()
            .last()
            .unwrap()
            .to_ascii_lowercase()
            .contains("x-account: fixture-a")
    );
    let before = serde_json::to_value(&worker.credentials.book).unwrap();
    let calls = provider.exchanges();
    let ledger = runtime::replay::verify_ledger_dir(&report.ledger_dir)
        .unwrap()
        .raw_lines()
        .to_vec();
    for (index, (secret, header)) in [
        (Some("secret:fixture/old"), None),
        (None, Some("X-Legacy")),
        (Some(""), None),
        (None, Some("")),
    ]
    .into_iter()
    .enumerate()
    {
        let result = worker.handle(wire::Command::ProbeEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url: base_url.clone(),
            dialect: kernel::DialectKind::OpenAi,
            secret: secret.map(str::to_owned),
            auth_header: header.map(str::to_owned),
            tuning: wire::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(
                &kernel::RunId::CITY,
                kernel::Seq::FIRST,
                format!("legacy-probe-{index}").as_bytes(),
            ),
        });
        assert_eq!(
            *result
                .expect_err("legacy fields have no account target")
                .code(),
            kernel::AxCode::ConfigInvalid
        );
        assert_eq!(
            serde_json::to_value(&worker.credentials.book).unwrap(),
            before
        );
        assert_eq!(provider.exchanges(), calls);
        assert_eq!(
            runtime::replay::verify_ledger_dir(&report.ledger_dir)
                .unwrap()
                .raw_lines(),
            ledger.as_slice()
        );
    }
    drop(worker);
    let resumed = super::RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&resumed.credentials.book).unwrap(),
        before
    );
}

#[test]
fn an_empty_ceiling_keeps_the_one_this_model_was_registered_with() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-1"], Vec::new());

    let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();

    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-1".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            input: None,
            idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"re-picked"),
        })
        .unwrap();
    let (_, _, entry) = worker.credentials.book.choices().next().unwrap();
    assert_eq!(
        entry.max_output_tokens,
        kernel::Ceiling::new(4_096),
        "re-picking a model erased the ceiling the person entered"
    );
    assert_eq!(entry.context_tokens, 32_768);
}

#[test]
fn a_chat_face_model_nobody_stated_a_ceiling_for_is_left_to_the_provider() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-1", "m-2"], Vec::new());
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-2".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: None,
            max_output_tokens: None,
            input: None,
            idem: kernel::IdemKey::derive(
                &kernel::RunId::CITY,
                kernel::Seq::FIRST,
                b"picked with nothing stated",
            ),
        })
        .unwrap();
    let (_, _, entry) = worker.credentials.book.choices().next().unwrap();
    assert_eq!(entry.id, "m-2");
    assert_eq!(entry.max_output_tokens, None);
}
#[test]
fn provider_accounts_keep_the_successful_session_after_reorder_and_restart() {
    use kernel::event::record::ProviderAccount;
    use kernel::{Address, IdemKey, RunId, Seq, ServerLabel};
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-1"],
        vec![
            crate::worker::fixture::completion("answered", None),
            crate::worker::fixture::completion("answered", None),
            crate::worker::fixture::completion("answered", None),
            crate::worker::fixture::completion("new session", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
    let accounts = ["one", "two"].map(|name| ProviderAccount {
        id: ServerLabel::parse(name).unwrap(),
        reference: Some(kernel::SecretRef::new("fixture", name).unwrap()),
        header: None,
    });
    let enrol = |worker: &mut crate::worker::RunWorker| {
        for name in ["one", "two"] {
            worker
                .handle(wire::Command::PutSecret {
                    realm: "fixture".to_owned(),
                    name: name.to_owned(),
                    value: kernel::Sealed::new(Box::new(format!("fixture-value-{name}"))),
                })
                .unwrap();
        }
    };
    enrol(&mut worker);
    let attach =
        |worker: &mut crate::worker::RunWorker, order: Vec<ProviderAccount>, marker: &[u8]| {
            worker
                .handle(wire::Command::AttachEndpoint {
                    name: wire::ProviderName::parse("house").unwrap(),
                    base_url: base_url.clone(),
                    dialect: kernel::DialectKind::OpenAi,
                    secret: None,
                    auth_header: None,
                    admit: vec!["m-1".to_owned()],
                    named: Vec::new(),
                    tuning: wire::EndpointTuning {
                        accounts: Some(order),
                        ..wire::EndpointTuning::default()
                    },
                    idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, marker),
                })
                .unwrap();
        };
    let dispatch = |worker: &mut crate::worker::RunWorker, marker: &[u8]| {
        worker
            .handle(wire::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: "answer once".to_owned(),
                goal: "one answer".to_owned(),
                policy: kernel::RunPolicy::of(kernel::Mode::Work),
                idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, marker),
                session: None,
                resume: None,
                images: Vec::new(),
                effort: None,
                model: None,
            })
            .unwrap();
    };
    attach(&mut worker, accounts.to_vec(), b"account-order-one");
    dispatch(&mut worker, b"account-run-one");
    let recorded =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
    let called = recorded
        .lines()
        .iter()
        .filter_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == kernel::EventKind::ModelCalled =>
            {
                Some(
                    record
                        .data()
                        .read::<kernel::event::record::ModelCalled>()
                        .unwrap()
                        .provider_account,
                )
            }
            runtime::replay::VerifiedLine::Known { .. }
            | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
        })
        .collect::<Vec<_>>();
    assert!(
        called.iter().any(|binding| binding
            .as_ref()
            .is_some_and(|binding| binding.account.as_str() == "one")),
        "production calls must record account identity: {called:?}"
    );
    assert_eq!(
        worker
            .credentials
            .book
            .session_account(&Address::parse("lab/room1").unwrap(), "house")
            .map(ServerLabel::as_str),
        Some("one"),
        "first successful call must enter the live binding projection"
    );
    attach(
        &mut worker,
        accounts.iter().rev().cloned().collect(),
        b"account-order-two",
    );
    dispatch(&mut worker, b"account-run-two");
    drop(worker);
    let mut worker = crate::worker::RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker.read_volume_with(crate::worker::fixture::roomy_volume);
    enrol(&mut worker);
    dispatch(&mut worker, b"account-run-three");
    worker
        .handle(wire::Command::OpenSession {
            addr: Address::parse("lab/room1").unwrap(),
            carry: wire::Carry::Nothing,
            from: None,
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"account-session-new"),
        })
        .unwrap();
    dispatch(&mut worker, b"account-run-four");
    let auth = provider
        .exchanges()
        .into_iter()
        .filter(|request| request.starts_with("POST "))
        .map(|request| {
            request
                .lines()
                .find(|line| line.to_ascii_lowercase().starts_with("authorization:"))
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        auth,
        [
            vec!["authorization: Bearer fixture-value-one".to_owned(); 3],
            vec!["authorization: Bearer fixture-value-two".to_owned()]
        ]
        .concat()
    );
}
