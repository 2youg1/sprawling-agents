// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::payload::{attached_payload, selected_payload};
use super::super::tuning::EndpointTuning;
use super::*;
use crate::endpoint::AuthSpec;
use crate::market::ModelEntry;
use kernel::{
    AxCode, DialectKind, EventKind, EventRecord, ModelTag, Payload, SecretRef, UsdMicros,
};
use kernel::{EventDraft, GENESIS_PREV, RunId, Seq, TimeMs};
use serde_json::Map;
use serde_json::Value;
fn record(kind: EventKind, data: Payload) -> EventRecord {
    EventRecord::from_draft(
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(1),
            who: "owner".to_owned(),
            addr: None,
            kind,
            data,
            ig: false,
        },
        Seq::FIRST,
        GENESIS_PREV,
    )
}

use crate::endpoint::ModelFacts;
use crate::provider::registry::ConnectionKind;

fn facts(id: &str) -> ModelFacts {
    ModelFacts {
        id: id.to_owned(),
        ..ModelFacts::default()
    }
}

fn attached(name: &str, base_url: &str) -> AttachedEndpoint {
    AttachedEndpoint {
        name: name.to_owned(),
        base_url: base_url.to_owned(),
        dialect: DialectKind::OpenAi,
        connection_kind: ConnectionKind::OpenAiCompat,
        auth: AuthSpec::Bearer(SecretRef::parse("secret:provider/key").unwrap()),
        models: vec![facts("m-small"), facts("m-large")],
        probed: true,
        tuning: EndpointTuning::default(),
        facts_blob: None,
    }
}
fn entry(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_owned(),
        context_tokens: 128_000,
        window_from: Some(crate::FigureSource::Person),
        max_output_tokens: kernel::Ceiling::new(8_192),
        ceiling_from: Some(crate::FigureSource::Person),
        input: crate::market::InputKinds::Text,
        input_price: UsdMicros::new(1_000_000),
        output_price: UsdMicros::new(2_000_000),
        cache_read_price: UsdMicros::new(0),
        cache_write_price: UsdMicros::new(0),
    }
}
fn book_with(base_url: &str) -> EndpointBook {
    let mut book = EndpointBook::new();
    let endpoint = attached("house", base_url);
    book.apply(&record(
        EventKind::EndpointAttached,
        attached_payload(&endpoint).unwrap(),
    ))
    .unwrap();
    book.apply(&record(
        EventKind::ModelSelected,
        selected_payload(ModelTag::Main, "house", &entry("m-large")).unwrap(),
    ))
    .unwrap();
    book
}

#[test]
fn max_in_flight_survives_a_replay_and_a_snapshot() {
    let mut capped = attached("capped", "https://api.example.test/v1");
    capped.tuning.max_in_flight = Some(crate::concurrency::MaxInFlight::try_from(3).unwrap());
    let mut book = EndpointBook::new();
    for endpoint in [
        capped,
        attached("remote", "https://api.example.test/v1"),
        attached("local", "http://127.0.0.1:11434/v1"),
    ] {
        let mut line = attached_payload(&endpoint).unwrap().as_map().clone();
        if endpoint.name != "capped" {
            assert!(!line.contains_key("tuning"), "{line:?}");
            line.remove("tuning");
        }
        book.apply(&record(
            EventKind::EndpointAttached,
            Payload::new(line).unwrap(),
        ))
        .unwrap();
    }
    let restored: EndpointBook =
        serde_json::from_str(&serde_json::to_string(&book).unwrap()).unwrap();
    let limits = |book: &EndpointBook| {
        ["capped", "remote", "local"].map(|name| book.endpoints[name].transport.limit())
    };
    assert_eq!((limits(&book), limits(&restored)), ([3, 16, 4], [3, 16, 4]));
}

#[test]
fn account_retries_survive_a_replay_and_an_unset_line_takes_the_default() {
    use kernel::account_recovery::AccountRetries;
    let mut chosen = attached("chosen", "https://api.example.test/v1");
    chosen.tuning.account_retries = Some(AccountRetries::One);
    let written = attached_payload(&chosen).unwrap();
    assert_eq!(
        written
            .as_map()
            .get("tuning")
            .and_then(|tuning| tuning.get("account_retries")),
        Some(&Value::String("one".to_owned()))
    );
    let mut unreadable = attached_payload(&attached("odd", "https://api.example.test/v1"))
        .unwrap()
        .as_map()
        .clone();
    unreadable.insert(
        "tuning".to_owned(),
        serde_json::json!({"account_retries": "three"}),
    );
    let mut book = EndpointBook::new();
    for line in [
        written,
        attached_payload(&attached("unset", "https://api.example.test/v1")).unwrap(),
        Payload::new(unreadable).unwrap(),
    ] {
        book.apply(&record(EventKind::EndpointAttached, line))
            .unwrap();
    }
    let read = ["chosen", "unset", "odd"].map(|name| {
        let tuning = &book.endpoints[name].endpoint.tuning;
        (tuning.account_retries, tuning.account_retries())
    });
    assert_eq!(
        read,
        [
            (Some(AccountRetries::One), AccountRetries::One),
            (None, EndpointTuning::DEFAULTS.account_retries),
            (None, EndpointTuning::DEFAULTS.account_retries),
        ]
    );
    assert_eq!(
        EndpointTuning::DEFAULTS.account_retries,
        AccountRetries::Two
    );
}

#[test]
fn a_record_naming_a_retreat_still_reads_as_the_choice_it_carries() {
    let mut book = EndpointBook::new();
    book.apply(&record(
        EventKind::EndpointAttached,
        attached_payload(&attached("house", "https://api.example.test/v1")).unwrap(),
    ))
    .unwrap();
    let mut map = selected_payload(ModelTag::Main, "house", &entry("m-large"))
        .unwrap()
        .as_map()
        .clone();
    map.insert(
        "fallback_endpoint".to_owned(),
        Value::String("spare".to_owned()),
    );
    map.insert(
        "fallback_model".to_owned(),
        Value::String("m-small".to_owned()),
    );
    book.apply(&record(
        EventKind::ModelSelected,
        Payload::new(map).unwrap(),
    ))
    .unwrap();
    let chosen = book
        .select(ModelTag::Main, &BuildingPolicy::default())
        .unwrap();
    assert_eq!(chosen.entry.id, "m-large");
}

#[test]
fn a_tag_with_no_choice_says_what_to_do_about_it() {
    let err = EndpointBook::new()
        .select(ModelTag::Digest, &BuildingPolicy::default())
        .unwrap_err();
    assert_eq!(*err.code(), AxCode::ModelUnchosen);
    assert!(err.recovery().contains("settings page"));
}

#[test]
fn a_confidential_building_cannot_choose_a_model_off_this_machine() {
    let remote = book_with("https://api.example.test/v1");
    assert!(
        remote
            .select(ModelTag::Main, &BuildingPolicy::default())
            .is_ok()
    );
    let err = remote
        .select(ModelTag::Main, &BuildingPolicy::new(true))
        .unwrap_err();
    assert_eq!(*err.code(), AxCode::GateDenied);

    let local = book_with("http://127.0.0.1:11434/v1");
    assert!(
        local
            .select(ModelTag::Main, &BuildingPolicy::new(true))
            .is_ok(),
        "a loopback endpoint is what a confidential building is allowed to reach"
    );
}

#[test]
fn losing_an_endpoint_takes_its_choices_with_it() {
    let mut book = book_with("https://api.example.test/v1");
    let mut gone = Map::new();
    gone.insert("name".to_owned(), Value::String("house".to_owned()));
    book.apply(&record(
        EventKind::EndpointLost,
        Payload::new(gone).unwrap(),
    ))
    .unwrap();
    assert!(book.is_empty());
    assert!(
        book.select(ModelTag::Main, &BuildingPolicy::default())
            .is_err()
    );
}

#[test]
fn the_dialect_owns_the_path_the_person_did_not_type() {
    let openai = attached("house", "https://api.example.test/v1/");
    assert_eq!(
        openai.chat_url(),
        "https://api.example.test/v1/chat/completions"
    );
    assert_eq!(openai.models_url(), "https://api.example.test/v1/models");
    let mut anthropic = attached("house", "https://api.example.test/v1");
    anthropic.dialect = DialectKind::Anthropic;
    assert_eq!(anthropic.chat_url(), "https://api.example.test/v1/messages");
}

#[test]
fn a_record_this_book_does_not_own_passes_through() {
    let mut book = EndpointBook::new();
    book.apply(&record(EventKind::RunStarted, Payload::empty()))
        .unwrap();
    assert!(book.is_empty());
}

#[test]
fn an_unreadable_registration_is_an_error_not_a_skip() {
    let mut book = EndpointBook::new();
    let mut half = Map::new();
    half.insert("name".to_owned(), Value::String("house".to_owned()));
    let err = book
        .apply(&record(
            EventKind::EndpointAttached,
            Payload::new(half).unwrap(),
        ))
        .unwrap_err();
    assert_eq!(*err.code(), AxCode::WireMismatch);
}
#[test]
fn malformed_provider_accounts_are_refused_during_replay() {
    let mut payload = attached_payload(&attached("house", "https://api.example.test/v1"))
        .unwrap()
        .as_map()
        .clone();
    payload.insert(
        "tuning".to_owned(),
        serde_json::json!({
            "accounts": [{"id": "first", "reference": "plain"}]
        }),
    );
    let mut book = EndpointBook::new();
    assert!(
        book.absorb(
            EventKind::EndpointAttached,
            RunId::CITY,
            None,
            &Payload::new(payload).unwrap()
        )
        .is_err(),
        "an invalid explicit account must not silently fall back to legacy auth"
    );
    assert!(book.is_empty());
}

fn accounts() -> Vec<kernel::event::record::ProviderAccount> {
    ["first", "second"]
        .map(|id| kernel::event::record::ProviderAccount {
            id: kernel::ServerLabel::parse(id).unwrap(),
            reference: Some(kernel::SecretRef::new("providers", id).unwrap()),
            header: None,
        })
        .to_vec()
}

#[test]
fn provider_accounts_preserve_explicit_reorder_and_reject_malformed_replay() {
    let mut endpoint = attached("house", "http://127.0.0.1:11434/v1");
    endpoint.tuning.accounts = Some(accounts());
    let mut book = EndpointBook::new();
    book.absorb(
        EventKind::EndpointAttached,
        RunId::CITY,
        None,
        &attached_payload(&endpoint).unwrap(),
    )
    .unwrap();
    endpoint.tuning.accounts.as_mut().unwrap().reverse();
    book.absorb(
        EventKind::EndpointAttached,
        RunId::CITY,
        None,
        &attached_payload(&endpoint).unwrap(),
    )
    .unwrap();
    endpoint.tuning.accounts = None;
    book.absorb(
        EventKind::EndpointAttached,
        RunId::CITY,
        None,
        &attached_payload(&endpoint).unwrap(),
    )
    .unwrap();
    let kept = book.endpoints().next().unwrap();
    assert_eq!(
        kept.tuning
            .accounts
            .as_ref()
            .unwrap()
            .iter()
            .map(|account| account.id.as_str())
            .collect::<Vec<_>>(),
        ["second", "first"]
    );
    assert_eq!(
        kept.first_auth().unwrap(),
        AuthSpec::Bearer(SecretRef::new("providers", "second").unwrap())
    );
    let encoded = serde_json::to_vec(&book).unwrap();
    let restored: EndpointBook = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&book).unwrap()
    );
    let mut payload = attached_payload(kept).unwrap().as_map().clone();
    payload.get_mut("tuning").unwrap()["accounts"][0]["reference"] = serde_json::json!("plain");
    assert!(
        book.absorb(
            EventKind::EndpointAttached,
            RunId::CITY,
            None,
            &Payload::new(payload).unwrap()
        )
        .is_err()
    );
}

#[test]
fn failed_account_attempts_never_replace_successful_session_affinity() {
    let mut book = EndpointBook::new();
    let addr = kernel::Address::parse("lab/session").unwrap();
    let run = RunId::from_bytes([9; 16]);
    book.absorb(EventKind::RunStarted, run, Some(&addr), &Payload::empty())
        .unwrap();
    let called = |account: &str| {
        Payload::of(&kernel::event::record::ModelCalled {
            model: "model".to_owned(),
            segments: Vec::new(),
            provider_account: Some(kernel::event::record::ProviderAccountBinding {
                provider: "house".to_owned(),
                account: kernel::ServerLabel::parse(account).unwrap(),
            }),
        })
        .unwrap()
    };
    book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("first"))
        .unwrap();
    assert!(book.session_account(&addr, "house").is_none());
    book.absorb(EventKind::ModelReturned, run, None, &Payload::empty())
        .unwrap();
    book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("second"))
        .unwrap();
    book.absorb(EventKind::RunFrozen, run, Some(&addr), &Payload::empty())
        .unwrap();
    assert_eq!(
        book.session_account(&addr, "house").unwrap().as_str(),
        "first"
    );
    let run = RunId::from_bytes([10; 16]);
    book.absorb(EventKind::RunStarted, run, Some(&addr), &Payload::empty())
        .unwrap();
    book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("second"))
        .unwrap();
    book.absorb(EventKind::ModelReturned, run, None, &Payload::empty())
        .unwrap();
    assert_eq!(
        book.session_account(&addr, "house").unwrap().as_str(),
        "second"
    );
    book.absorb(
        EventKind::SessionOpened,
        RunId::CITY,
        Some(&addr),
        &Payload::empty(),
    )
    .unwrap();
    assert!(book.session_account(&addr, "house").is_none());
}
#[test]
fn old_run_answer_must_not_bind_a_new_session() {
    let mut book = EndpointBook::new();
    let room = kernel::Address::parse("lab/room1").unwrap();
    let old_run = RunId::from_bytes([2; 16]);
    book.absorb(
        EventKind::RunStarted,
        old_run,
        Some(&room),
        &Payload::empty(),
    )
    .unwrap();
    let called = Payload::of(&kernel::event::record::ModelCalled {
        model: "fixture".to_owned(),
        segments: Vec::new(),
        provider_account: Some(kernel::event::record::ProviderAccountBinding {
            provider: "house".to_owned(),
            account: kernel::ServerLabel::parse("old").unwrap(),
        }),
    })
    .unwrap();
    book.absorb(EventKind::ModelCalled, old_run, None, &called)
        .unwrap();
    book.absorb(
        EventKind::SessionOpened,
        RunId::CITY,
        Some(&room),
        &Payload::empty(),
    )
    .unwrap();
    book.absorb(EventKind::ModelReturned, old_run, None, &Payload::empty())
        .unwrap();
    assert_eq!(book.session_account(&room, "house"), None);
}
proptest::proptest! {
    #[test]
    fn failed_attempt_traces_preserve_the_lean_binding(
        attempted in proptest::collection::vec(0u8..8, 0..32),
    ) {
        let mut book = EndpointBook::new();
        let addr = kernel::Address::parse("lab/session").unwrap();
        let run = RunId::from_bytes([9; 16]);
        book.absorb(EventKind::RunStarted, run, Some(&addr), &Payload::empty()).unwrap();
        let called = |id: &str| Payload::of(&kernel::event::record::ModelCalled {
            model: "fixture".to_owned(), segments: Vec::new(),
            provider_account: Some(kernel::event::record::ProviderAccountBinding {
                provider: "house".to_owned(), account: kernel::ServerLabel::parse(id).unwrap(),
            }),
        }).unwrap();
        book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("bound")).unwrap();
        book.absorb(EventKind::ModelReturned, run, None, &Payload::empty()).unwrap();
        for id in attempted {
            book.absorb(EventKind::ModelCalled, run, Some(&addr), &called(&format!("account{id}"))).unwrap();
            proptest::prop_assert_eq!(book.session_account(&addr, "house").unwrap().as_str(), "bound");
        }
        book.absorb(EventKind::RunFrozen, run, Some(&addr), &Payload::empty()).unwrap();
        proptest::prop_assert_eq!(book.session_account(&addr, "house").unwrap().as_str(), "bound");
    }
}
