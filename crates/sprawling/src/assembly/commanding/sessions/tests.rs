// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Starting a new session: the refusal that protects a running one, and
//! the two answers to what a new session keeps.
//!
//! The room's queue is what says a run is working there, so the refusal
//! test lends it the way a dispatch does rather than inventing a second
//! book of who is busy. The two carrying tests read the ledger line the
//! verb wrote, because that line is the only place the answer outlives
//! the call: both of the facts it records are removals.

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

mod origin;

pub(super) fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

/// Whether the history recorded a `session_opened` at this address, and
/// what it said about carrying.
fn carried_in_history(city_root: &Path, at: &Address) -> Option<bool> {
    let verified =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(city_root).ledger())
            .unwrap();
    verified
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .find(|line| line["kind"] == "session_opened" && line["addr"] == at.as_str())
        .map(|line| line["data"]["carried"].as_bool().unwrap())
}

/// A room whose first session froze a shape, and which has a summary the
/// last one left for the next.
fn frozen_room(city_root: &Path, room: &Address) -> std::path::PathBuf {
    city::write_session(city_root, room, "m-local", Some(kernel::Effort::High)).unwrap();
    let handoff = city::handoff_path(city_root, room);
    std::fs::create_dir_all(handoff.parent().unwrap()).unwrap();
    std::fs::write(&handoff, "the previous session's summary\n").unwrap();
    handoff
}

/// A run working in the room is the one refusal: replacing the session
/// under it would let two stretches claim one prefix.
#[test]
fn a_run_working_in_the_room_refuses_a_new_session() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let handoff = frozen_room(dir.path(), &room);

    // What a dispatch does for the length of its run: take the room's
    // queue. Nothing else in the city says a room is being worked in.
    let running = kernel::RunId::from_bytes([9u8; 16]);
    let _lent = worker.collaborating.rooms.lend(&room, running);

    let refused = worker
        .open_session(&room, channels::Carry::Nothing, None)
        .unwrap_err();
    assert_eq!(refused.code(), &kernel::AxCode::Busy);
    assert!(
        refused.subject().contains(&running.to_string()),
        "the refusal names the run to stop: {}",
        refused.subject()
    );
    assert!(
        refused.recovery().contains("/stop") && refused.recovery().contains("/new"),
        "and the two verbs in the order that works: {}",
        refused.recovery()
    );
    // The refusal costs the person nothing: the shape is still frozen
    // and the summary is still there, so a retry after `/stop` is the
    // same session start it would have been.
    assert_eq!(
        city::own_layer(dir.path(), &room).unwrap().model(),
        Some("m-local")
    );
    assert!(handoff.exists());
    assert_eq!(carried_in_history(dir.path(), &room), None);
}

/// The default: a new session chooses its own shape and carries no
/// summary, and the line says so.
#[test]
fn a_new_session_carries_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let handoff = frozen_room(dir.path(), &room);

    worker
        .open_session(&room, channels::Carry::Nothing, None)
        .unwrap();

    assert_eq!(city::own_layer(dir.path(), &room).unwrap().model(), None);
    assert!(!handoff.exists(), "the slot is emptied, not blanked");
    assert_eq!(carried_in_history(dir.path(), &room), Some(false));
}

/// `--carry`: the summary travels, and the shape is forgotten anyway -
/// which is what made the room dispatchable again.
#[test]
fn carrying_the_summary_still_lets_the_room_choose_a_model() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let handoff = frozen_room(dir.path(), &room);

    worker
        .open_session(&room, channels::Carry::Handoff, None)
        .unwrap();

    assert_eq!(city::own_layer(dir.path(), &room).unwrap().model(), None);
    assert!(handoff.exists(), "this is what the person asked to keep");
    assert_eq!(carried_in_history(dir.path(), &room), Some(true));
}

/// A person who says `--carry` in a room with no summary is not refused
/// and is not lied to: the record says the session carried nothing.
#[test]
fn carrying_nothing_is_not_a_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    city::write_session(dir.path(), &room, "m-local", Some(kernel::Effort::High)).unwrap();

    worker
        .open_session(&room, channels::Carry::Handoff, None)
        .unwrap();

    assert_eq!(carried_in_history(dir.path(), &room), Some(false));
}

/// A branch: the session's first run opens with the mother's own
/// conversation, and the second run of that session does not.
///
/// What is asserted is what the provider was asked, because that is the
/// only place the inheritance is visible: the messages are not in the
/// prefix, so no segment hash records them, and the ledger holds the
/// mother's lines rather than a copy of what the child sent. The body of
/// the first request is the fact.
#[test]
fn a_branching_session_opens_with_the_mothers_conversation() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("the meter says 42", None),
            completion("the second run", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = addr("lab/room1");
    let elsewhere = addr("lab/room2");

    // The mother: one turn, in another room, whose every word is in the
    // branch's first request.
    worker
        .handle(channels::Command::Dispatch {
            addr: elsewhere.clone(),
            task: "measure the meter".to_owned(),
            goal: "a number is written down".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"mother"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let verified =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
    let mother = verified
        .lines()
        .iter()
        .find_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunStarted =>
            {
                Some((record.run(), record.seq()))
            }
            runtime::replay::VerifiedLine::Known { .. }
            | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
        })
        .expect("the mother ran");
    let last = verified.tail_seq().unwrap();

    // The branch: a session at another room whose first run inherits.
    worker
        .handle(channels::Command::OpenSession {
            addr: room.clone(),
            carry: channels::Carry::Nothing,
            from: Some(kernel::Origin {
                run: mother.0,
                at_seq: last,
            }),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"branch"),
        })
        .unwrap();
    let before = provider.bodies().len();
    worker
        .handle(channels::Command::Dispatch {
            addr: room.clone(),
            task: "carry on".to_owned(),
            goal: "the work goes on".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"carry-on"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    let bodies = provider.bodies();
    let first = &bodies[before];
    assert!(
        first.contains("measure the meter"),
        "the branch's first request opens with the mother's task: {}",
        first.chars().take(400).collect::<String>()
    );
    assert!(
        first.contains("the meter says 42"),
        "and with what the mother answered"
    );
    let messages: serde_json::Value = serde_json::from_str(first).unwrap();
    assert!(
        messages["messages"].as_array().unwrap().len() >= 3,
        "task, reply and the new task: {}",
        messages["messages"]
    );

    // The second run of the same session inherits nothing: the branch is
    // a beginning, and the run that began it was the first one.
    let after = provider.bodies().len();
    worker
        .handle(channels::Command::Dispatch {
            addr: room.clone(),
            task: "again".to_owned(),
            goal: "once more".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"again"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let next = provider.bodies();
    assert!(
        !next[after].contains("the meter says 42"),
        "the inheritance is spent by the run that began the session"
    );
}

/// `--carry` keeps the conversation's address as well as the summary:
/// the session's first run is told where the previous run's transcript
/// is, as a successor handed the work down is, because the summary
/// says what was found and only the transcript says how.
#[test]
fn a_carried_session_names_the_previous_runs_transcript() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![completion("first", None), completion("second", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = addr("lab/room1");
    let dispatch = |task: &str, key: &[u8]| channels::Command::Dispatch {
        addr: room.clone(),
        task: task.to_owned(),
        goal: "done".to_owned(),
        model: None,
        mode: kernel::Mode::PlanGoal,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        session: None,
        effort: None,
    };
    worker.handle(dispatch("measure", b"one")).unwrap();
    let verified =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
    let previous = verified
        .lines()
        .iter()
        .find_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunStarted =>
            {
                Some(record.run())
            }
            runtime::replay::VerifiedLine::Known { .. }
            | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
        })
        .expect("the first run started");
    std::fs::write(
        city::handoff_path(dir.path(), &room),
        "<overall>\nMeasuring.\n</overall>\n",
    )
    .unwrap();
    worker
        .handle(channels::Command::OpenSession {
            addr: room.clone(),
            carry: channels::Carry::Handoff,
            from: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"new"),
        })
        .unwrap();
    let before = provider.bodies().len();
    worker.handle(dispatch("carry on", b"two")).unwrap();

    let pointer = format!("Predecessor transcript: lab/room1/{previous}.jsonl");
    let first = &provider.bodies()[before];
    assert!(
        first.contains(&pointer),
        "the carried session's first request names {pointer}: {}",
        first.chars().take(600).collect::<String>()
    );
}
