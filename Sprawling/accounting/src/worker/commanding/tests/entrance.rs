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

use crate::worker::Posted;
use crate::worker::fixture::*;
use crate::worker::*;

fn rooms_under(city_root: &Path, building: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(city_root.join(building))
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            (name != ".sprawling").then_some(name)
        })
        .collect();
    names.sort();
    names
}

fn lines_of(ledger_dir: &Path, kind: &str) -> usize {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter(|value| value["kind"] == kind)
        .count()
}

#[test]
fn the_same_dispatch_twice_under_one_key_opens_one_room_and_starts_one_run() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let asked = || wire::Command::Dispatch {
        addr: Address::parse("lab").unwrap(),
        task: "read the plan".to_owned(),
        goal: "one answer".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"lab|read the plan"),
        session: Some(kernel::SessionName::parse("one").unwrap()),
        resume: None,
        images: Vec::new(),
        effort: None,
        model: None,
    };
    for _sent in 0..2 {
        worker.serve_one(Posted {
            command: asked(),
            reply: wire::Reply::nowhere(),
        });
    }

    worker.land_the_rest().unwrap();

    assert_eq!(
        rooms_under(dir.path(), "lab"),
        vec!["one".to_owned()],
        "one ask opened one room, whatever the socket did"
    );
    assert_eq!(
        lines_of(&report.ledger_dir, "run_started"),
        1,
        "and the history says the work was taken once"
    );
}

#[test]
fn a_repeat_is_answered_with_what_the_first_ask_was_answered() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();

    let heard: Arc<std::sync::Mutex<Vec<AxError>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let peer = || {
        let told = Arc::clone(&heard);
        wire::Reply::to(move |error| {
            let Ok(mut told) = told.lock() else {
                return wire::Delivered::PeerGone;
            };
            told.push(error);
            wire::Delivered::ToThePeer
        })
    };

    let asked = || wire::Command::Cancel {
        run: RunId::CITY,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"cancel"),
    };
    for _sent in 0..2 {
        worker.serve_one(Posted {
            command: asked(),
            reply: peer(),
        });
    }
    let told = heard.lock().unwrap();
    assert_eq!(told.len(), 2, "both asks are answered; neither is dropped");
    assert_eq!(
        told[0], told[1],
        "the second ask gets the first answer, word for word"
    );

    drop(told);
    let halt = || wire::Command::Halt {
        scope: wire::HaltScope::City,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"halt"),
    };
    for _sent in 0..2 {
        worker.serve_one(Posted {
            command: halt(),
            reply: peer(),
        });
    }
    assert_eq!(
        heard.lock().unwrap().len(),
        2,
        "the repeated halt was not refused: it had already been done"
    );
    assert_eq!(
        lines_of(&report.ledger_dir, "city_halted"),
        1,
        "and the city was halted once, not twice"
    );
}

#[test]
fn a_key_already_in_the_history_is_recognised_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let halt = || wire::Command::Halt {
        scope: wire::HaltScope::City,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"halt"),
    };
    {
        let mut worker = RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap();
        worker.serve_one(Posted {
            command: halt(),
            reply: wire::Reply::nowhere(),
        });
    }
    let mut restarted = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    restarted.serve_one(Posted {
        command: halt(),
        reply: wire::Reply::nowhere(),
    });
    assert_eq!(
        lines_of(&report.ledger_dir, "city_halted"),
        1,
        "the key was on the ledger, so the restarted city knew it had done this"
    );
}
