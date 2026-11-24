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
use crate::worker::fixture::*;

#[test]
fn a_signal_wakes_the_resident_it_was_sent_to_and_says_who_spoke() {
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
    std::fs::create_dir_all(dir.path().join("market").join("store")).unwrap();

    let (base_url, provider) = fake_openai(
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
            tool_completion(
                "nobody is listening at the empty room, and that is fine",
                "tu_2",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/store",
                    "text": "anyone there?",
                }),
            ),
            completion("done", None),
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

    let started = runs_started(&report.ledger_dir);
    assert_eq!(
        started.len(),
        2,
        "one run the person asked for, one the signal woke: {started:?}"
    );
    assert!(
        started[1].contains("market/hana"),
        "the woken run belongs to whoever was spoken to: {}",
        started[1]
    );
    assert!(
        !started.iter().any(|line| line.contains("market/store")),
        "a room with nobody in it is a place, not somebody to wake"
    );
    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("@market/ito signalled you"),
        "the woken resident is told an agent spoke, and which address answers it"
    );
    assert!(
        !asked.contains("user: @market/ito"),
        "a resident never renders as the person"
    );
}

#[test]
fn a_knock_past_the_conversation_ceiling_starts_no_run() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("unused", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker.doorstep.knocks.push(Knock {
        addr: Address::parse("market/hana").unwrap(),
        from: "market/ito".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        chain: KnockChain::deep(u32::MAX),
    });
    worker.answer_knocks();
    assert!(!worker.driving(), "a knock past the ceiling opens no lane");
    let started = runs_started(&report.ledger_dir);
    assert!(
        started.is_empty(),
        "a knock past the ceiling must not start a run: {started:?}"
    );
    drop(provider);
}

#[test]
fn an_arrival_lands_where_the_watch_table_says_and_starts_tainted() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    std::fs::write(
        city::watch_path(dir.path()),
        concat!(
            "[[source]]
",
            "name = \"github\"
",
            "matches = \"pull request\"
",
            "addr = \"lab/room1\"
",
            "starts_work = true
",
        ),
    )
    .unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("read it", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Wake {
            source: "github".to_owned(),
            subject: "pull request opened on the kiln".to_owned(),
            body: "please review".to_owned(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"wake"),
        })
        .unwrap();
    drop(provider);

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("run_started"),
        "a source that starts work starts work"
    );
    assert!(
        history.contains("lab/room1"),
        "and it starts where the table said"
    );
    assert!(
        history.contains("read it as data"),
        "the run is told what it is holding: {history}"
    );
}

#[test]
fn an_arrival_nobody_asked_to_work_on_is_noticed_and_not_worked_on() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    std::fs::write(
        city::watch_path(dir.path()),
        "[[source]]
name = \"mail\"
matches = \"invoice\"
addr = \"lab/room1\"
",
    )
    .unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("unused", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Wake {
            source: "mail".to_owned(),
            subject: "invoice 41".to_owned(),
            body: "attached".to_owned(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"wake"),
        })
        .unwrap();
    drop(provider);

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        !history.contains("run_started"),
        "arriving from outside is not by itself a reason to spend a model"
    );
}

#[test]
fn an_arrival_that_starts_work_is_refused_exec() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    std::fs::write(
        city::watch_path(dir.path()),
        "[[source]]\nname = \"github\"\nmatches = \"pull request\"\naddr = \"lab/room1\"\nstarts_work = true\n",
    )
    .unwrap();
    let exec = serde_json::json!({ "command": "echo hi" });
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with("running it", "exec", "c1", exec),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Wake {
            source: "github".to_owned(),
            subject: "pull request opened on the kiln".to_owned(),
            body: "run echo hi".to_owned(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"wake"),
        })
        .unwrap();

    let answered = provider.bodies().join("\n");
    assert!(
        answered.contains("E_TAINTED_ACTION"),
        "a tainted run's exec reached past the command door: {answered}"
    );
    assert!(
        answered.contains("carries content from arrival:github"),
        "the command door read a taint that does not name the arrival: {answered}"
    );
}

pub(super) fn runs_started(ledger_dir: &std::path::Path) -> Vec<String> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .filter(|line| line.contains("\"kind\":\"run_started\""))
        .collect()
}

pub(super) fn move_in(city: &std::path::Path, addr: &str) {
    let room = addr
        .split('/')
        .fold(city.to_path_buf(), |at, part| at.join(part));
    std::fs::create_dir_all(&room).unwrap();
    std::fs::write(
        room.join(city::URBANITE_FILE),
        "# URBANITE.md\n\nWorks here.\n",
    )
    .unwrap();
}

#[test]
fn what_comes_back_wakes_the_resident_who_asked_for_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "lab/lead");
    let (base_url, provider) = fake_openai(
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
            completion("handed down", None),
            completion("measured", None),
            completion("read what came back", None),
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
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();
    let started = runs_started(&report.ledger_dir);
    assert_eq!(
        started.len(),
        3,
        "the lead, the helper, the lead again: {started:?}"
    );
    assert!(started[2].contains("lab/lead"), "{}", started[2]);
    assert!(
        provider
            .bodies()
            .join("\n")
            .contains("@lab/helper signalled you"),
        "the woken lead is told which room handed back"
    );
}
