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

//! What a signal does the moment it is sent, and the knock that waits
//! for a working room to empty (collab D7, D11).

use super::super::*;
use super::tests::{move_in, runs_started};
use crate::worker::fixture::*;

/// D7: a signal takes effect when it is sent. Its `signal_enqueued`
/// line reaches the history while the sender is still running, before
/// that run's `run_frozen`, rather than when the sender lands.
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

/// A knock at a room whose queue is out with a run starts nothing: a
/// second run there would read a spare inbox that nothing is delivered
/// into, while the signal waits for the holder. When the holder gives
/// the queue back the same knock goes out, and the run it starts finds
/// the signal at home (`crates/sprawling/Spec.lean` §8-46-12).
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
    // What the knock is for: delivered while the room is lent, so it
    // waits in the holder's slot and comes home with the queue.
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
