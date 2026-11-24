// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::super::*;
use super::tests::{move_in, runs_started};
use crate::worker::fixture::*;

#[test]
fn a_signal_is_on_the_history_before_the_sender_freezes() {
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
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "asking hana",
                "tu_1",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/hana",
                    "text": "what is your rate?",
                }),
            ),
            completion("done", None),
            completion("hana answers", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "ask hana what she charges".to_owned(),
            goal: "a price".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();
    let kinds: Vec<String> = runtime::replay::verify_ledger_dir(&report.ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .map(|line| line["kind"].as_str().unwrap_or_default().to_owned())
        .collect();
    let sent = kinds.iter().position(|kind| kind == "signal_enqueued");
    let frozen = kinds.iter().position(|kind| kind == "run_frozen");
    assert!(
        matches!((sent, frozen), (Some(sent), Some(frozen)) if sent < frozen),
        "the signal is sent while its sender runs: {kinds:?}"
    );
}

#[test]
fn a_knock_at_a_room_somebody_is_working_in_waits_for_them_to_leave() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    move_in(dir.path(), "market/hana");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("answered", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let hana = Address::parse("market/hana").unwrap();
    let working = RunId::from_bytes([7u8; 16]);
    let lent = worker.collaborating.rooms.lend(&hana, working);
    worker
        .collaborating
        .rooms
        .deliver(
            &collab::Signal::new(
                kernel::event::record::SignalId::parse("s-1").unwrap(),
                kernel::event::record::SignalKind::Mention,
                "market/ito".to_owned(),
                hana.clone(),
                kernel::Version::new(1),
                kernel::Payload::empty(),
                kernel::TimeMs::new(1),
            )
            .unwrap(),
        )
        .unwrap();
    worker.doorstep.knocks.push(Knock {
        addr: hana.clone(),
        from: "market/ito".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        chain: KnockChain::default(),
    });
    worker.answer_knocks();
    worker.land_the_rest().unwrap();
    assert_eq!(
        runs_started(&report.ledger_dir),
        Vec::<String>::new(),
        "nobody is woken in a room somebody is already working in"
    );
    worker.vacate(&hana, working, lent.inbox).unwrap();
    worker.answer_knocks();
    worker.land_the_rest().unwrap();
    let started = runs_started(&report.ledger_dir);
    assert_eq!(
        started.len(),
        1,
        "the knock goes out once the room is empty: {started:?}"
    );
    assert!(started[0].contains("market/hana"), "{}", started[0]);
}

#[test]
fn a_signal_to_an_empty_room_starts_its_run_before_the_sender_freezes() {
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
    let calls = std::sync::Arc::new((std::sync::Mutex::new(0u32), std::sync::Condvar::new()));
    let pace: Pace = std::sync::Arc::new(move |request: &str| {
        if !request.starts_with("POST ") {
            return;
        }
        let (count, arrived) = &*calls;
        let mut count = count.lock().unwrap();
        *count += 1;
        arrived.notify_all();
        if *count == 2 {
            drop(
                arrived
                    .wait_timeout_while(count, std::time::Duration::from_secs(30), |seen| *seen < 3)
                    .unwrap(),
            );
        }
    });
    let (base_url, _provider) = fake_openai_paced(
        &["m-local"],
        Vec::new(),
        vec![
            tool_completion(
                "asking hana",
                "tu_1",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/hana",
                    "text": "what is your rate?",
                }),
            ),
            completion("done", None),
            completion("hana answers", None),
        ],
        pace,
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "ask hana what she charges".to_owned(),
            goal: "a price".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();
    worker.land_the_rest().unwrap();
    let lines: Vec<serde_json::Value> = runtime::replay::verify_ledger_dir(&report.ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .collect();
    let at = |kind: &str, room: &str| {
        lines.iter().position(|line| {
            line["kind"] == kind && line.to_string().contains(&format!("\"{room}\""))
        })
    };
    let (woken, frozen) = (
        at("run_started", "market/hana"),
        at("run_frozen", "market/ito"),
    );
    assert!(
        matches!((woken, frozen), (Some(woken), Some(frozen)) if woken < frozen),
        "the knock goes out at delivery: {woken:?} {frozen:?}"
    );
}

enum Receiver {
    Working,

    Unoccupied,

    Idle,
}

fn landings_after_a_send(receiver: Receiver) -> Vec<(String, Option<String>)> {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    move_in(dir.path(), "market/ito");
    let hana = Address::parse("market/hana").unwrap();
    match receiver {
        Receiver::Working | Receiver::Idle => move_in(dir.path(), "market/hana"),
        Receiver::Unoccupied => {}
    }
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "asking hana",
                "tu_1",
                "signal",
                serde_json::json!({ "action": "send", "to": "market/hana", "text": "rate?" }),
            ),
            completion("done", None),
            completion("hana answers", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let _lent = match receiver {
        Receiver::Working => Some(
            worker
                .collaborating
                .rooms
                .lend(&hana, RunId::from_bytes([7u8; 16])),
        ),
        Receiver::Unoccupied | Receiver::Idle => None,
    };
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "ask hana what she charges".to_owned(),
            goal: "a price".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();
    let lines: Vec<serde_json::Value> = runtime::replay::verify_ledger_dir(&report.ledger_dir)
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
            let landing = lines
                .iter()
                .filter(|line| {
                    line["kind"] == "signal_landed" && line["data"]["signal"] == id.as_str()
                })
                .map(|line| line["data"]["landing"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            assert!(
                landing.len() <= 1,
                "one landing line per signal: {landing:?}"
            );
            (id, landing.into_iter().next())
        })
        .collect()
}

#[test]
fn a_sent_letter_says_where_it_landed() {
    for (receiver, expected) in [
        (Receiver::Working, "delivered"),
        (Receiver::Unoccupied, "queued"),
        (Receiver::Idle, "knocked"),
    ] {
        let landings = landings_after_a_send(receiver);
        assert_eq!(landings.len(), 1, "{landings:?}");
        assert_eq!(
            landings
                .iter()
                .map(|(_, landing)| landing.as_deref())
                .collect::<Vec<_>>(),
            vec![Some(expected)],
            "{landings:?}"
        );
    }
}
