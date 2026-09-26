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
use crate::assembly::fixture::*;

/// A resident who is signalled and has no run open gets one, and its
/// brief names the resident who spoke rather than reading like the
/// person. Two residents can therefore hold a conversation without
/// somebody dispatching each turn of it by hand.
#[test]
fn a_signal_wakes_the_resident_it_was_sent_to_and_says_who_spoke() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("market").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "market/ito");
    move_in(dir.path(), "market/hana");
    // A room with nobody in it, to prove the other half of the rule.
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
        .handle(channels::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "ask hana what she charges".to_owned(),
            goal: "a price".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
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

/// A knock that would carry one conversation past its ceiling starts
/// nothing: the signal is already in the room's inbox, and the chain
/// ends here rather than with another run nobody asked for
/// (sprawling-SPEC.md 8-46-12). The first test above is the other side
/// of the boundary: a knock below the ceiling wakes its resident.
#[test]
fn a_knock_past_the_conversation_ceiling_starts_no_run() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("unused", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker.doorstep.knocks.push(Knock {
        addr: Address::parse("market/hana").unwrap(),
        from: "market/ito".to_owned(),
        mode: kernel::Mode::PlanGoal,
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
    let report = init_city(dir.path()).unwrap();
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
        .handle(channels::Command::Wake {
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
    let report = init_city(dir.path()).unwrap();
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
        .handle(channels::Command::Wake {
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

/// Outside content that starts work starts it tainted, and the taint
/// has to reach the door that judges `exec`: a run a web page or a pull
/// request set going must not run a command on the city's machine.
///
/// Through the production path - arrival, dispatch, the bench the
/// workbench lays out - because the defect this pins was a bench that
/// never learned the run was tainted, which no unit of the bench shows.
/// The refusal names the source the work arrived from, so the taint the
/// door read is the arrival's own label rather than one made up later.
#[test]
fn an_arrival_that_starts_work_is_refused_exec() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    std::fs::write(
        city::watch_path(dir.path()),
        "[[source]]\nname = \"github\"\nmatches = \"pull request\"\naddr = \"lab/room1\"\nstarts_work = true\n",
    )
    .unwrap();
    let exec = serde_json::json!({ "arm": { "shell": { "text": "echo hi" } } });
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with("running it", "exec", "c1", exec),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Wake {
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

/// Every `run_started` line the history holds, oldest first.
fn runs_started(ledger_dir: &std::path::Path) -> Vec<String> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .filter(|line| line.contains("\"kind\":\"run_started\""))
        .collect()
}

/// Gives the room at `addr` somebody to wake.
fn move_in(city: &std::path::Path, addr: &str) {
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

/// A knock at a room whose queue is out with a run starts nothing: a
/// second run there would read a spare inbox that nothing is delivered
/// into, while the signal waits for the holder. When the holder gives
/// the queue back the same knock goes out, and the run it starts finds
/// the signal at home (sprawling-SPEC.md 8-46-12).
#[test]
fn a_knock_at_a_room_somebody_is_working_in_waits_for_them_to_leave() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    move_in(dir.path(), "market/hana");
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("answered", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let hana = Address::parse("market/hana").unwrap();
    let working = RunId::from_bytes([7u8; 16]);
    let lent = worker.collaborating.rooms.lend(&hana, working);
    worker.doorstep.knocks.push(Knock {
        addr: hana.clone(),
        from: "market/ito".to_owned(),
        mode: kernel::Mode::PlanGoal,
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

/// What a delegate hands back wakes the resident who asked for it, by
/// the same decision an ordinary signal takes: the brief names the room
/// that spoke, so the parent's next turn is a run of its own rather
/// than a person dispatching it again (sprawling-SPEC.md 8-46-12).
#[test]
fn what_comes_back_wakes_the_resident_who_asked_for_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
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
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/lead").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
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
