// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_goal_that_lands_on_a_claimed_path_is_refused_with_the_level_that_decides_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let claim = serde_json::json!({
        "statement": "rewrite the kiln notes",
        "paths": ["lab/room1/notes.md"],
        "standing": true,
    });
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion("claiming", "tu_1", "goal", claim.clone()),
            completion("claimed", None),
            tool_completion("claiming too", "tu_2", "goal", claim),
            completion("gave way", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for (n, room) in ["lab/room1", "lab/room2"].into_iter().enumerate() {
        worker
            .handle(wire::Command::Dispatch {
                addr: Address::parse(room).unwrap(),
                task: "claim the notes".to_owned(),
                goal: "register a goal, then stop".to_owned(),
                policy: kernel::RunPolicy::of(kernel::Mode::Work),
                idem: kernel::IdemKey::derive(
                    &RunId::CITY,
                    kernel::Seq::new(u64::try_from(n).unwrap()),
                    b"dispatch",
                ),
                session: None,
                effort: None,
                model: None,
            })
            .unwrap();
    }

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("goal_registered"),
        "the first claim stands"
    );
    assert!(
        history.contains("goal_conflict"),
        "the second one is a fact about the city, not only a refusal the model saw"
    );
    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("E_GOAL_CONFLICT"),
        "the refusal reaches the model that asked"
    );
}

/// Two runs dispatched side by side read one goal register. The
/// accounting thread decides each registration at the call, so the
/// second run to stake the same ground is refused before it spends
/// another call, and the history holds the one registration and the
/// clash (sprawling-SPEC.md 8-42-8).
#[test]
fn two_runs_registering_one_ground_side_by_side_leave_one_holder() {
    use kernel::Tool;
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let tool = |run: u8, room: &str| {
        let desk = worker.goal_desk(
            RunId::from_bytes([run; 16]),
            &Address::parse(room).unwrap(),
            &format!("potter@lab.{run}"),
        );
        collab::GoalTool::new(
            Address::parse(room).unwrap(),
            std::sync::Arc::new(std::sync::Mutex::new(desk)),
        )
        .unwrap()
    };
    let (first, second) = (tool(1, "lab/room1"), tool(2, "lab/room2"));
    let call = kernel::ToolCall {
        id: "tu_1".to_owned(),
        name: kernel::ToolName::parse("goal").unwrap(),
        args: Payload::new(
            serde_json::json!({
                "statement": "rewrite the kiln notes",
                "paths": ["lab/room1/notes.md"],
                "standing": true,
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .unwrap(),
    };
    let lanes = std::thread::spawn(move || {
        let taken = first.invoke(&call).is_ok();
        (
            taken,
            second.invoke(&call).map_err(|refusal| *refusal.code()),
        )
    });
    while !lanes.is_finished() {
        worker
            .serve_flight(relay::Patience::For(std::time::Duration::from_millis(5)))
            .unwrap();
    }
    let (taken, refused) = lanes.join().unwrap();
    let goal_lines: Vec<String> = worker
        .ledger
        .read_raw_lines()
        .unwrap()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter_map(|line| line["kind"].as_str().map(str::to_owned))
        .filter(|kind| kind.starts_with("goal_"))
        .collect();
    assert_eq!(
        (
            taken,
            refused.err(),
            goal_lines,
            worker.collaborating.goals.len()
        ),
        (
            true,
            Some(AxCode::GoalConflict),
            vec!["goal_registered".to_owned(), "goal_conflict".to_owned()],
            1
        ),
        "the first run holds the ground from its call, and the second is refused at its own"
    );
}

/// A standing goal on a building with no plan would find no ready step
/// and finish at once having done nothing, so it is refused instead, with
/// the subject the client's form recovery reads.
#[test]
fn a_standing_goal_on_a_building_without_a_plan_is_refused_with_the_building_and_the_goal() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let refusal = worker
        .handle(wire::Command::Pursue {
            addr: Address::parse("lab").unwrap(),
            step: wire::PursuitStep::Set {
                goal: "fire the kiln".to_owned(),
            },
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"pursue"),
        })
        .unwrap_err();

    assert_eq!(
        (refusal.code().as_str(), refusal.subject()),
        ("E_PLAN_MISSING", "lab: fire the kiln")
    );
}

/// A plan that is there but does not parse is a different fact from no
/// plan: the refusal names the broken line instead of asking the mayor
/// to write a plan over the one the person already wrote.
#[test]
fn a_standing_goal_on_a_malformed_plan_is_refused_with_the_broken_line() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        "| id | two columns |
",
    )
    .unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let refusal = worker
        .handle(wire::Command::Pursue {
            addr: Address::parse("lab").unwrap(),
            step: wire::PursuitStep::Set {
                goal: "fire the kiln".to_owned(),
            },
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"pursue"),
        })
        .unwrap_err();

    assert_eq!(
        (
            refusal.code().as_str(),
            refusal.subject().contains("line 1")
        ),
        ("E_INVALID_ARGS", true)
    );
}

/// Setting a pursuit starts its rows and gives the desk back: `Pause`,
/// `Halt` and every other command are read by the main loop, and a
/// pursuit that drove its rows to the end inside the command kept them
/// all out. The rows still land, through the same loop.
#[test]
fn a_pursuit_gives_the_desk_back_while_its_rows_drive() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        PLAN_ONE_FREE_ROW,
    )
    .unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("wire-the-kiln", None), completion("done", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"digest"),
        })
        .unwrap();
    let lab = Address::parse("lab").unwrap();
    worker
        .set_pursuit(
            &lab,
            wire::PursuitStep::Set {
                goal: "fire the kiln".to_owned(),
            },
        )
        .unwrap();
    assert!(
        worker.driving(),
        "the pursuit returned before its row came home"
    );
    worker.land_the_rest().unwrap();
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let frozen = verified
        .raw_lines()
        .iter()
        .filter(|line| String::from_utf8_lossy(line).contains("\"kind\":\"run_frozen\""))
        .count();
    assert_eq!(frozen, 1, "the row still landed");
}
