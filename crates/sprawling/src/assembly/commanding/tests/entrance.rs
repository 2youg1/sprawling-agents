// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The key every command carries, read at the door it enters by.
//!
//! What these pin: the wire makes all 23 state-changing commands carry
//! an `IdemKey` so that a retry is harmless, and until this door read it
//! nothing in the city did. `serving::desk` collapses a repeat that is
//! still queued or still running; a repeat that arrives after the first
//! one finished used to be a second effect.

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

use crate::assembly::fixture::*;
use crate::assembly::*;
use crate::serving::Posted;

/// Every room that has been opened under one building.
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

/// How many lines of one kind the history carries.
fn lines_of(ledger_dir: &Path, kind: &str) -> usize {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter(|value| value["kind"] == kind)
        .count()
}

/// The defect: the same command sent twice under one key happened
/// twice. A client that sent a dispatch, lost its socket, and sent it
/// again got two rooms and two runs out of one piece of work.
#[test]
fn the_same_dispatch_twice_under_one_key_opens_one_room_and_starts_one_run() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n",
    );
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    // One key, minted once, sent twice - which is exactly what a client
    // that retried a frame it was not sure had arrived would send.
    let asked = || channels::Command::Dispatch {
        addr: Address::parse("lab").unwrap(),
        task: "read the plan".to_owned(),
        goal: "one answer".to_owned(),
        mode: channels::ModeTag::parse("plan").unwrap(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"lab|read the plan"),
        session: Some(kernel::SessionName::parse("one").unwrap()),
        effort: None,
    };
    for _sent in 0..2 {
        worker.serve_one(Posted {
            command: asked(),
            reply: channels::Reply::nowhere(),
        });
    }
    // The desk starts a run and the loop lands it; this is that loop.
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

/// A repeat is answered with the first answer, not with a second one
/// that the first effect made up. Answering an approval twice used to
/// report "nothing is waiting" the second time - a refusal manufactured
/// by the success of the first attempt, which tells a retrying client
/// that its command failed when it succeeded.
#[test]
fn a_repeat_is_answered_with_what_the_first_ask_was_answered() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    let heard: Arc<std::sync::Mutex<Vec<AxError>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let peer = || {
        let told = Arc::clone(&heard);
        channels::Reply::to(move |error| {
            let Ok(mut told) = told.lock() else {
                return channels::Delivered::PeerGone;
            };
            told.push(error);
            channels::Delivered::ToThePeer
        })
    };
    // A run id nothing answers to: refused every time, and refused for
    // the same reason every time.
    let asked = || channels::Command::Cancel {
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

    // A command that succeeded is not carried out twice either, and the
    // second ask is not told that nothing was waiting.
    drop(told);
    let halt = || channels::Command::Halt {
        scope: channels::HaltScope::City,
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

/// The key survives the process. A city that restarts folds its history
/// and finds the keys of every command that left a record in it, so a
/// client retrying across a restart is still asking for one thing.
#[test]
fn a_key_already_in_the_history_is_recognised_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let halt = || channels::Command::Halt {
        scope: channels::HaltScope::City,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"halt"),
    };
    {
        let mut worker = RunWorker::new(
            dir.path(),
            gateway::Custodian::in_memory(),
            runtime::diagnostics::Diagnostics::off(),
        )
        .unwrap();
        worker.serve_one(Posted {
            command: halt(),
            reply: channels::Reply::nowhere(),
        });
    }
    let mut restarted = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    restarted.serve_one(Posted {
        command: halt(),
        reply: channels::Reply::nowhere(),
    });
    assert_eq!(
        lines_of(&report.ledger_dir, "city_halted"),
        1,
        "the key was on the ledger, so the restarted city knew it had done this"
    );
}
