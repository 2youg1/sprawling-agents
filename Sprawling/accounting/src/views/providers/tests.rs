// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::config::SearchSupplier;

#[test]
fn an_unstated_second_rung_is_answered_with_the_default_as_its_layer() {
    let dir = tempfile::tempdir().unwrap();
    let addr = Address::parse("lab/room1").unwrap();
    let answer = serde_json::to_value(config_answer(dir.path(), &addr, None).unwrap()).unwrap();
    assert_eq!(
        answer.get("second"),
        Some(&serde_json::json!({
            "percent": kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT,
            "from": "default",
            "domain": {
                "min": kernel::consts_policy::CTX_REMINDER_SECOND_MIN,
                "max": kernel::consts_policy::CTX_REMINDER_SECOND_MAX,
            },
        }))
    );
}

#[test]
fn the_first_rung_is_answered_beside_the_second() {
    let dir = tempfile::tempdir().unwrap();
    let addr = Address::parse("lab/room1").unwrap();
    let answer = config_answer(dir.path(), &addr, None).unwrap();
    assert_eq!(
        answer.first,
        Some(kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT)
    );
}

#[test]
fn the_tuning_defaults_are_answered_with_the_default_as_their_layer() {
    let dir = tempfile::tempdir().unwrap();
    let addr = Address::parse("lab/room1").unwrap();
    let answer = serde_json::to_value(config_answer(dir.path(), &addr, None).unwrap()).unwrap();
    assert_eq!(
        answer.pointer("/tuning/from"),
        Some(&serde_json::json!("default"))
    );
}

fn account(id: &str, keyed: bool) -> ProviderAccount {
    ProviderAccount {
        id: kernel::ServerLabel::parse(id).unwrap(),
        reference: keyed.then(|| kernel::SecretRef::new("fixture", id).unwrap()),
        header: keyed.then(|| "x-api-key".to_owned()),
    }
}

#[test]
fn every_key_state_is_read_from_what_the_vault_describes() {
    let mut vault = gateway::Custodian::in_memory().with_env_reader(Box::new(|key| {
        if key.ends_with("SHADED") {
            Ok("sk-from-the-environment".to_owned())
        } else if key.ends_with("SPOILT") {
            Err(std::env::VarError::NotUnicode(std::ffi::OsString::new()))
        } else {
            Err(std::env::VarError::NotPresent)
        }
    }));
    vault
        .set(
            &kernel::SecretRef::new("fixture", "stored").unwrap(),
            zeroize::Zeroizing::new("sk-stored".to_owned()),
        )
        .unwrap();
    let listed = [
        account("stored", true),
        account("missing", true),
        account("shaded", true),
        account("spoilt", true),
        account("open", false),
    ];
    let state = |id: &str, key| wire::AccountStatus {
        id: kernel::ServerLabel::parse(id).unwrap(),
        key,
    };

    assert_eq!(
        (statuses(&listed, Some(&vault)), statuses(&listed, None)),
        (
            vec![
                state("stored", wire::KeyState::Stored),
                state("missing", wire::KeyState::Missing),
                state("shaded", wire::KeyState::Environment),
                state("spoilt", wire::KeyState::EnvironmentUnusable),
                state("open", wire::KeyState::Anonymous),
            ],
            vec![
                state("stored", wire::KeyState::Unread),
                state("missing", wire::KeyState::Unread),
                state("shaded", wire::KeyState::Unread),
                state("spoilt", wire::KeyState::Unread),
                state("open", wire::KeyState::Anonymous),
            ],
        )
    );
}

#[test]
fn the_search_answer_names_the_city_value_apart_from_a_building_override() {
    let dir = tempfile::tempdir().unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let custom = SearchConfiguration::Custom {
        selected: kernel::ServerLabel::parse("brave").unwrap(),
        suppliers: vec![SearchSupplier {
            id: kernel::ServerLabel::parse("brave").unwrap(),
            url: "https://search.example/mcp".to_owned(),
            remote: "brave_web_search".to_owned(),
            query_field: "query".to_owned(),
            objective_field: None,
            count_field: Some("count".to_owned()),
            accounts: vec![account("main", true)],
        }],
    };
    city::write_search(dir.path(), &custom).unwrap();
    let building =
        kernel::layout::CityLayout::new(dir.path()).config(&Address::parse("lab").unwrap());
    std::fs::create_dir_all(building.parent().unwrap()).unwrap();
    std::fs::write(&building, "[search]\nchoice = \"off\"\n").unwrap();
    let vault = Arc::new(Mutex::new(gateway::Custodian::in_memory()));

    let answered = config_answer(dir.path(), &room, Some(&vault)).unwrap();

    assert_eq!(
        answered.search,
        wire::SettledSearch {
            configuration: SearchConfiguration::Off,
            from: wire::ConfigLayer::Building,
            city: Some(custom),
            default_url: city::default_search_supplier().unwrap().url,
            account_status: vec![wire::SupplierAccounts {
                supplier: kernel::ServerLabel::parse("brave").unwrap(),
                accounts: vec![wire::AccountStatus {
                    id: kernel::ServerLabel::parse("main").unwrap(),
                    key: wire::KeyState::Missing,
                }],
            }],
        }
    );
}

#[test]
fn a_relay_forwarding_a_vendors_id_answers_the_vendors_window() {
    let book = gateway::EndpointBook::new();
    let sonnet = [wire::FiguresAsked {
        id: "claude-sonnet-4-5".to_owned(),
        context_tokens: None,
        max_output_tokens: None,
        tag: None,
    }];
    let ask = |base_url: &str, face| {
        figures_answer(
            &book,
            &TableRows {
                endpoint: "relay",
                base_url,
                face,
                models: &sonnet,
            },
        )
    };
    let line = |window: Option<u64>, ceiling, input| {
        wire::Answer::ModelFigures(wire::ModelFiguresAnswer {
            models: vec![wire::FiguresLine {
                id: "claude-sonnet-4-5".to_owned(),
                window: window
                    .and_then(kernel::Window::new)
                    .map(|tokens| wire::WindowLine {
                        tokens,
                        from: wire::FigureFrom::Preset,
                    }),
                ceiling: Some(ceiling),
                input,
            }],
        })
    };
    let vendor = wire::CeilingLine::Sent {
        tokens: kernel::Ceiling::new(64_000).unwrap(),
        from: wire::FigureFrom::Preset,
    };
    assert_eq!(
        [
            ask("relay.example.test/v1", DialectKind::OpenAi),
            ask("https://relay.example.test/v1", DialectKind::Anthropic),
            ask("http://127.0.0.1:8811/v1", DialectKind::OpenAi),
        ],
        [
            line(
                Some(200_000),
                wire::CeilingLine::ProviderDefault,
                kernel::InputKinds::TextImage,
            ),
            line(Some(200_000), vendor, kernel::InputKinds::TextImage),
            line(
                None,
                wire::CeilingLine::ProviderDefault,
                kernel::InputKinds::Text,
            ),
        ]
    );
    assert!(matches!(
        ask("mailto:somebody", DialectKind::OpenAi),
        wire::Answer::Unavailable {
            reason: Some(_),
            ..
        }
    ));
}

#[test]
fn each_rung_travels_under_the_word_the_ledger_records() {
    let sources = [
        gateway::FigureSource::Person,
        gateway::FigureSource::Upstream,
        gateway::FigureSource::Registered,
        gateway::FigureSource::Preset,
        gateway::FigureSource::Policy,
    ];
    assert_eq!(
        sources.map(|source| serde_json::to_value(figure_from(source)).unwrap()),
        sources.map(|source| serde_json::json!(source.as_str()))
    );
}
