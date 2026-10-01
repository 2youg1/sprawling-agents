// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The names a request carries are the names the page reads: held for
//! the whole session they were frozen in, and taken anew after `/new`
//! (accounting-SPEC.md 8-15).

#![allow(clippy::wildcard_enum_match_arm, reason = "test code")]

use std::path::Path;

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

fn idem(tag: &str) -> kernel::IdemKey {
    kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, tag.as_bytes())
}

/// Names the Mayor through the card, from the text the page last read.
fn call_the_mayor(worker: &mut RunWorker, city: &Path, name: &str) {
    let base = std::fs::read_to_string(city::Governed::Mayor.path(city)).unwrap_or_default();
    worker
        .handle(wire::Command::PutIdentity {
            card: wire::IdentityCard::Mayor {
                name: Some(name.to_owned()),
            },
            base,
            idem: idem(name),
        })
        .unwrap();
}

fn dispatch(worker: &mut RunWorker, tag: &str) {
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "say who runs the city".to_owned(),
            goal: String::new(),
            policy: kernel::RunPolicy::of(kernel::Mode::Chat),
            idem: idem(tag),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
}

/// What the page reads: the Mayor's name and the version a new session
/// freezes, through the same query a page sends.
fn page(city: &Path) -> (Option<String>, kernel::B3Hash) {
    let wire::Answer::Identity(answer) = crate::views::ask(city, &wire::Query::Identity).unwrap()
    else {
        panic!("the identity query answered something else")
    };
    let wire::IdentityAnswer::Stated(stated) = *answer else {
        panic!("the identity areas did not read: {answer:?}")
    };
    (stated.mayor, stated.version)
}

/// The identity version every `run_started` line recorded, in order.
fn frozen_versions(city: &Path) -> Vec<Option<kernel::B3Hash>> {
    let ledger = kernel::layout::CityLayout::new(city).ledger();
    runtime::replay::verify_ledger_dir(&ledger)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| kernel::EventRecord::parse_line(line).unwrap())
        .filter(|record| record.kind() == kernel::EventKind::RunStarted)
        .map(|record| {
            record
                .data()
                .read::<kernel::event::record::RunStarted>()
                .unwrap()
                .naming
        })
        .collect()
}

/// The name on the settings card is the name in the request the model is
/// sent, under one version both sides can point at. A rename leaves the
/// running session as it was frozen - its next request still says the
/// old name - and the session `/new` opens takes the new one.
#[test]
fn a_new_session_freezes_the_name_the_page_shows() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("the Mayor", None),
            completion("still the Mayor", None),
            completion("a new Mayor", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: idem("create"),
        })
        .unwrap();

    call_the_mayor(&mut worker, dir.path(), "Cat");
    let (shown, cat) = page(dir.path());
    assert_eq!(shown.as_deref(), Some("Cat"));
    dispatch(&mut worker, "first");

    call_the_mayor(&mut worker, dir.path(), "Dog");
    let (shown, dog) = page(dir.path());
    assert_eq!(shown.as_deref(), Some("Dog"));
    assert_ne!(cat, dog, "a rename is a new version");
    dispatch(&mut worker, "second");

    worker
        .handle(wire::Command::OpenSession {
            addr: Address::parse("lab/room1").unwrap(),
            carry: wire::Carry::Nothing,
            from: None,
            idem: idem("new"),
        })
        .unwrap();
    dispatch(&mut worker, "third");

    // The chat requests, in order; the endpoint's model list was asked
    // for first, with no body.
    let bodies: Vec<String> = provider
        .bodies()
        .into_iter()
        .filter(|body| body.contains("\"messages\""))
        .collect();
    let said = |at: usize, name: &str| {
        bodies
            .get(at)
            .unwrap()
            .contains(&format!("is called {name}."))
    };
    assert!(said(0, "Cat"), "the first request: {}", bodies[0]);
    assert!(
        said(1, "Cat") && !said(1, "Dog"),
        "a rename reached a session that was already running: {}",
        bodies[1]
    );
    assert!(said(2, "Dog"), "the session /new opened: {}", bodies[2]);
    assert_eq!(
        frozen_versions(dir.path()),
        vec![Some(cat), Some(cat), Some(dog)],
        "run_started names the version each request was frozen with"
    );
}
