// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! Kernel D38 on the two paths that do not cross the relay: a child's
//! handback and a settled landing's delivery each write one
//! `signal_landed`, paired to its letter by `SignalId`
//! (`crates/collab/spec/Delivery.lean` property 8).

use super::super::*;
use super::tests::move_in;
use crate::worker::fixture::*;

/// Every `signal_enqueued` id in the ledger, each with the landings of
/// the `signal_landed` lines that name it.
fn landings(ledger_dir: &std::path::Path) -> Vec<(String, Vec<String>)> {
    let lines: Vec<serde_json::Value> = runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .collect();
    lines
        .iter()
        .filter(|line| line["kind"] == "signal_enqueued")
        .map(|sent| {
            let id = sent["data"]["id"].as_str().unwrap().to_owned();
            let landed = lines
                .iter()
                .filter(|line| {
                    line["kind"] == "signal_landed" && line["data"]["signal"] == id.as_str()
                })
                .map(|line| line["data"]["landing"].as_str().unwrap().to_owned())
                .collect();
            (id, landed)
        })
        .collect()
}

fn letter(id: &str, to: &str) -> collab::SignalEffect {
    collab::SignalEffect::Enqueued(
        collab::Signal::new(
            kernel::event::record::SignalId::parse(id).unwrap(),
            kernel::event::record::SignalKind::Mention,
            "ito".to_owned(),
            Address::parse(to).unwrap(),
            kernel::Version::new(1),
            kernel::Payload::empty(),
            crate::Clock::now(&crate::worker::fixture::WallClock).unwrap(),
        )
        .unwrap(),
    )
}

/// A settled landing delivers to an idle resident, which it knocks for,
/// and to a room nobody lives in, where the letter waits in the queue.
#[test]
fn a_settled_landing_says_where_each_letter_landed() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("market").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "market/ito");
    move_in(dir.path(), "market/hana");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let ito = Address::parse("market/ito").unwrap();
    let at = Assignment {
        addr: ito.clone(),
        parent: None,
        succession: None,
        session: None,
        effort: None,
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        taint: kernel::TaintSet::empty(),
        dispatched_by: kernel::event::Who::Person,
        origin: None,
    };
    let landing = crate::effect::Landing::signals(
        vec![letter("s-1", "market/hana"), letter("s-2", "market/kai")],
        &ito,
        "ito",
    )
    .unwrap();
    worker
        .settle(&at, RunId::CITY, landing, &KnockChain::default())
        .unwrap();
    assert_eq!(
        landings(&report.ledger_dir),
        vec![
            ("s-1".to_owned(), vec!["knocked".to_owned()]),
            ("s-2".to_owned(), vec!["queued".to_owned()]),
        ]
    );
}

/// A delegated child lands and hands back: the handback letter to the
/// parent's room has exactly one landing, like the letter that asked.
#[test]
fn a_handback_says_where_it_landed() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "lab/lead");
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "handing it down",
                "tu_1",
                "delegate",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion("handed down, carrying on", None),
            completion("measured", None),
            completion("read what came back", None),
            completion("nothing more", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/lead").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    worker.land_the_rest().unwrap();
    let landings = landings(&report.ledger_dir);
    assert!(
        landings.iter().any(|(id, _)| id.starts_with("handback-")),
        "the child handed back: {landings:?}"
    );
    assert!(
        landings.iter().all(|(_, landed)| landed.len() == 1
            && ["delivered", "queued", "knocked"].contains(&landed[0].as_str())),
        "one landing per letter: {landings:?}"
    );
}
