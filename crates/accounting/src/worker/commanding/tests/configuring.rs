// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::panic, reason = "test code")]

//! The city's own layer written from the settings page through
//! `ConfigureCity`, and read back through `Query::Config`
//! (`crates/wire/spec/Command/Step.lean` §8-61).

use kernel::config::{SearchConfiguration, SearchSupplier};
use kernel::{Address, IdemKey, RunId, Seq};

use crate::worker::*;

/// A unified building save judges its references before changing the catalog beside them.
#[test]
fn a_refused_connector_reference_leaves_the_whole_building_selection_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = opened(dir.path());
    let lab = Address::parse("lab").unwrap();
    city::create_building(dir.path(), &lab, city::BuildingTemplate::Minimal).unwrap();
    let file = kernel::layout::CityLayout::new(dir.path()).config(&lab);
    city::write_catalog(
        dir.path(),
        &lab,
        &kernel::config::ExposureTable {
            tools: [("exec".to_owned(), kernel::config::Exposure::Core)].into(),
            namespaces: std::collections::BTreeMap::new(),
            skills: std::collections::BTreeMap::new(),
        },
    )
    .unwrap();
    let before = std::fs::read(&file).unwrap();
    let settings = wire::BuildingSettings {
        addr: lab,
        sandbox: None,
        mcp: None,
        desktop: None,
        context_second_threshold: None,
        catalog: Some(kernel::config::ExposureTable {
            tools: [("exec".to_owned(), kernel::config::Exposure::Off)].into(),
            namespaces: std::collections::BTreeMap::new(),
            skills: std::collections::BTreeMap::new(),
        }),
        mcp_references: Some(vec![kernel::McpReference::legacy(
            kernel::ServerLabel::parse("absent").unwrap(),
        )]),
        skill_pins: None,
        reading_room: None,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"invalid-reference"),
    };

    let refused = worker
        .handle(wire::Command::ConfigureBuilding(settings))
        .unwrap_err();

    assert_eq!(refused.code(), &kernel::AxCode::ConfigInvalid);
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

/// A city whose own file is still what the city was opened with.
fn opened(root: &std::path::Path) -> RunWorker {
    crate::worker::fixture::init_city(root).unwrap();
    RunWorker::new(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap()
}

/// One supplier the page lists, keyed by a vault reference, and the
/// supplier `selected` names, which need not be it.
fn custom(selected: &str) -> SearchConfiguration {
    SearchConfiguration::Custom {
        selected: kernel::ServerLabel::parse(selected).unwrap(),
        suppliers: vec![SearchSupplier {
            id: kernel::ServerLabel::parse("brave").unwrap(),
            url: "https://search.example/mcp".to_owned(),
            remote: "brave_web_search".to_owned(),
            query_field: "query".to_owned(),
            objective_field: None,
            count_field: Some("count".to_owned()),
            accounts: vec![kernel::event::record::ProviderAccount {
                id: kernel::ServerLabel::parse("main").unwrap(),
                reference: Some(kernel::SecretRef::new("search", "brave.main").unwrap()),
                header: Some("x-api-key".to_owned()),
            }],
        }],
    }
}

fn configure(search: SearchConfiguration, tag: &[u8]) -> wire::Command {
    wire::Command::ConfigureCity(wire::CitySettings {
        keep_warm: Some(kernel::KeepWarm::FiveMinute),
        effort: None,
        search: Some(search),
        catalog: None,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, tag),
    })
}

/// The page's `[search]` lands in the city's own file, governs a room
/// that states nothing, and comes back as the value the page edits; the
/// key nobody filed reads as not looked up, since no vault was lent.
#[test]
fn a_city_search_setting_lands_in_the_city_layer_and_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = opened(dir.path());

    worker
        .handle(configure(custom("brave"), b"custom"))
        .unwrap();
    let wire::Answer::Config(answer) = crate::views::ask(
        dir.path(),
        &wire::Query::Config {
            addr: Address::parse("lab/room1").unwrap(),
        },
    )
    .unwrap() else {
        panic!("the configuration ladder is answered");
    };

    assert_eq!(
        answer.search,
        wire::SettledSearch {
            configuration: custom("brave"),
            from: wire::ConfigLayer::City,
            city: Some(custom("brave")),
            default_url: city::default_search_supplier().unwrap().url,
            account_status: vec![wire::SupplierAccounts {
                supplier: kernel::ServerLabel::parse("brave").unwrap(),
                accounts: vec![wire::AccountStatus {
                    id: kernel::ServerLabel::parse("main").unwrap(),
                    key: wire::KeyState::Unread,
                }],
            }],
        }
    );
}

/// A `[search]` the city refuses refuses the whole frame before a byte
/// is written: the keep-warm pick beside it does not land either.
#[test]
fn a_refused_search_setting_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = opened(dir.path());
    let file = kernel::layout::CityLayout::new(dir.path()).city_config();
    // A city fresh from its genesis may have no file of its own yet.
    let before = std::fs::read(&file).ok();

    let refused = worker
        .handle(configure(custom("unlisted"), b"unlisted"))
        .unwrap_err();

    assert_eq!(
        (*refused.code(), std::fs::read(&file).ok()),
        (kernel::AxCode::ConfigInvalid, before)
    );
}

/// A `[catalog]` the city refuses refuses the whole frame as well, even
/// when the `[search]` beside it is one the city takes: neither lands.
#[test]
fn a_refused_catalog_writes_nothing_beside_an_accepted_search() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = opened(dir.path());
    let file = kernel::layout::CityLayout::new(dir.path()).city_config();
    let before = std::fs::read(&file).ok();
    let wire::Command::ConfigureCity(settings) = configure(custom("brave"), b"catalog") else {
        panic!("a frame for the city's own layer");
    };

    let refused = worker
        .handle(wire::Command::ConfigureCity(wire::CitySettings {
            catalog: Some(kernel::config::ExposureTable {
                tools: [("Not A Tool".to_owned(), kernel::config::Exposure::Off)].into(),
                namespaces: std::collections::BTreeMap::new(),
                skills: std::collections::BTreeMap::new(),
            }),
            ..settings
        }))
        .unwrap_err();

    assert_eq!(
        (*refused.code(), std::fs::read(&file).ok()),
        (kernel::AxCode::ConfigInvalid, before)
    );
}
