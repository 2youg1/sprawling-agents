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

use crate::worker::fixture::*;
use crate::worker::*;

mod origin;

pub(super) fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

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

fn frozen_room(city_root: &Path, room: &Address) -> std::path::PathBuf {
    city::write_session(
        city_root,
        room,
        &city::SessionShape::new("m-local", Some(kernel::Effort::High), None),
    )
    .unwrap();
    let handoff = city::handoff_path(city_root, room);
    std::fs::create_dir_all(handoff.parent().unwrap()).unwrap();
    std::fs::write(&handoff, "the previous session's summary\n").unwrap();
    handoff
}

#[test]
fn a_run_working_in_the_room_refuses_a_new_session() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let handoff = frozen_room(dir.path(), &room);

    let running = kernel::RunId::from_bytes([9u8; 16]);
    let _lent = worker.collaborating.rooms.lend(&room, running);

    let refused = worker
        .open_session(&room, wire::Carry::Nothing, None)
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

    assert_eq!(
        city::own_layer(dir.path(), &room).unwrap().model(),
        Some("m-local")
    );
    assert!(handoff.exists());
    assert_eq!(carried_in_history(dir.path(), &room), None);
}

#[test]
fn a_new_session_carries_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let handoff = frozen_room(dir.path(), &room);

    worker
        .open_session(&room, wire::Carry::Nothing, None)
        .unwrap();

    assert_eq!(city::own_layer(dir.path(), &room).unwrap().model(), None);
    assert!(!handoff.exists(), "the slot is emptied, not blanked");
    assert_eq!(carried_in_history(dir.path(), &room), Some(false));
}

#[test]
fn carrying_the_summary_still_lets_the_room_choose_a_model() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let handoff = frozen_room(dir.path(), &room);

    worker
        .open_session(&room, wire::Carry::Handoff, None)
        .unwrap();

    assert_eq!(city::own_layer(dir.path(), &room).unwrap().model(), None);
    assert!(handoff.exists(), "this is what the person asked to keep");
    assert_eq!(carried_in_history(dir.path(), &room), Some(true));
}

#[test]
fn carrying_nothing_is_not_a_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/refactor");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    city::write_session(
        dir.path(),
        &room,
        &city::SessionShape::new("m-local", Some(kernel::Effort::High), None),
    )
    .unwrap();

    worker
        .open_session(&room, wire::Carry::Handoff, None)
        .unwrap();

    assert_eq!(carried_in_history(dir.path(), &room), Some(false));
}

#[test]
fn a_branching_session_opens_with_the_mothers_conversation() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
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

    worker
        .handle(wire::Command::Dispatch {
            addr: elsewhere.clone(),
            task: "measure the meter".to_owned(),
            goal: "a number is written down".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"mother"),
            session: None,
            resume: None,
            images: Vec::new(),
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

    worker
        .handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: Some(kernel::Origin {
                run: mother.0,
                at_seq: last,
            }),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"branch"),
        })
        .unwrap();
    let before = provider.bodies().len();
    worker
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "carry on".to_owned(),
            goal: "the work goes on".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"carry-on"),
            session: None,
            resume: None,
            images: Vec::new(),
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

    let after = provider.bodies().len();
    worker
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "again".to_owned(),
            goal: "once more".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"again"),
            session: None,
            resume: None,
            images: Vec::new(),
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

#[test]
fn a_carried_session_names_the_previous_runs_transcript() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![completion("first", None), completion("second", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = addr("lab/room1");
    let dispatch = |task: &str, key: &[u8]| wire::Command::Dispatch {
        addr: room.clone(),
        task: task.to_owned(),
        goal: "done".to_owned(),
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        session: None,
        resume: None,
        images: Vec::new(),
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
        .handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Handoff,
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

#[test]
fn an_explicit_past_target_appends_to_the_same_session_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("older selection marker", None),
            completion("recent session marker", None),
            completion("historical reply", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = addr("lab/room1");
    let dispatch = |task: &str, resume: Option<kernel::Seq>| wire::Command::Dispatch {
        addr: room.clone(),
        task: task.to_owned(),
        goal: String::new(),
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Chat),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, task.as_bytes()),
        session: None,
        resume,
        images: Vec::new(),
        effort: None,
    };
    worker.handle(dispatch("original question", None)).unwrap();
    let first = read_session_lines(dir.path(), &room)[0].began;
    worker
        .handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"new"),
        })
        .unwrap();
    worker.handle(dispatch("unrelated question", None)).unwrap();
    let before = read_session_lines(dir.path(), &room);
    assert_eq!(before.len(), 2);

    worker
        .handle(dispatch("historical follow-up", Some(first)))
        .unwrap();
    drop(worker);
    let after = read_session_lines(dir.path(), &room);
    assert_eq!(after.len(), 2);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[1].began, first);
    assert_eq!(after[1].runs, 2);
    assert_eq!(after[1].preview.as_deref(), Some("historical reply"));
    let bodies = provider.bodies();
    let last = bodies.last().unwrap();
    assert!(
        last.contains("older selection marker"),
        "original tail missing: {last}"
    );
    assert!(
        !last.contains("recent session marker"),
        "current session bled across: {last}"
    );
}

fn read_session_lines(root: &Path, room: &Address) -> Vec<wire::SessionLine> {
    let ledger =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(root).ledger())
            .unwrap();
    let mut projection = crate::views::sessions::RoomSessions::default();
    for line in ledger.lines() {
        if let runtime::replay::VerifiedLine::Known { record, .. } = line {
            projection.absorb(record).unwrap();
        }
    }
    projection.answer(room).sessions
}
