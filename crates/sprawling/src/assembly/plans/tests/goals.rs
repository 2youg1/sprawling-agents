// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

#[test]
fn a_goal_that_lands_on_a_claimed_path_is_refused_with_the_level_that_decides_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
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
            .handle(channels::Command::Dispatch {
                addr: Address::parse(room).unwrap(),
                task: "claim the notes".to_owned(),
                goal: "register a goal, then stop".to_owned(),
                mode: channels::ModeTag::parse("plan").unwrap(),
                idem: kernel::IdemKey::derive(
                    &RunId::CITY,
                    kernel::Seq::new(u64::try_from(n).unwrap()),
                    b"dispatch",
                ),
                session: None,
                effort: None,
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

/// A graph of nodes runs in dependency order, each in its own room
/// with its contract as its `JOB.md`, and what comes back verified
/// joins - so the next run in that room can be asked a question only
/// somebody who opened the results can answer.
///
/// `collab::workshop` and `collab::fanin` had no callers outside
/// their own files before this; the whole layer was a set of types
/// nobody had run.
#[test]
fn a_workshop_runs_its_nodes_in_order_and_what_comes_back_joins() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let graph = serde_json::json!({
        "op": "lay_out",
        "nodes": [
            {
                "room": "lab/writer",
                "goal": "write it up",
                "done_check": "the page exists",
                "stop": "when the page exists",
                "depends_on": ["lab/reader"],
            },
            {
                "room": "lab/reader",
                "goal": "read the meter",
                "done_check": "a number is written down",
                "stop": "when the number is written down",
            },
        ],
    });
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with("splitting it up", "workshop", "tu_1", graph.clone()),
            completion("waiting on a person", None),
            completion_with("splitting it up", "workshop", "tu_2", graph),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: room.clone(),
            task: "get it measured and written up".to_owned(),
            goal: "a page with a number in it, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    // Laying out a graph is a spawn like any other: the person is
    // asked once, and their answer carries the whole graph.
    let cluster = allow_the_one_pending_item(&mut worker);
    assert_eq!(cluster.class, kernel::ApprovalClass::Delegation);

    for node in ["lab/reader", "lab/writer"] {
        assert!(
            city::job_path(dir.path(), &Address::parse(node).unwrap()).exists(),
            "{node} was never given a job file"
        );
    }
    let contract = std::fs::read_to_string(city::job_path(
        dir.path(),
        &Address::parse("lab/reader").unwrap(),
    ))
    .unwrap();
    assert!(
        contract.contains("## Done check") && contract.contains("## Stop"),
        "a node's job file is its contract: {contract}"
    );

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    let reader = history.find("lab/reader").expect("the first node ran");
    let writer = history.find("lab/writer").expect("the second node ran");
    assert!(
        reader < writer,
        "the node everything waits on has to go first"
    );
    assert_eq!(
        worker
            .joins
            .get(&room)
            .map_or(0, |join| join.artifacts().count()),
        2,
        "both results joined, verified by the city rather than by their own producers"
    );
}

/// **Card 3.5's property.** A pursuit takes the whole ready set: three
/// nodes nothing blocks are three runs going at once, not three runs one
/// after another.
///
/// The assertion is on the history rather than on a stopwatch: with a
/// sequential pursuit each run's lines are a contiguous block, so
/// exactly one run has started by the time the first one freezes. Three
/// runs started before the first freeze is a fact only concurrency can
/// produce, and it is the fact a person is promised.
///
/// The three naming calls are spent on the accounting thread before any
/// lane is handed a drive, so the scripted replies below are consumed in
/// the order they are written; the turns that follow are concurrent and
/// all get the same last reply, which is what keeps this test's provider
/// script order-free where the order is not this test's to decide.
#[test]
fn three_ready_nodes_drive_three_runs_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    lay_rules(
        dir.path(),
        "lab",
        "# BUILDING.md\n\n`confidential: false`\n",
    );
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        PLAN_THREE_FREE_ROWS,
    )
    .unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("wire-the-kiln", None),
            completion("glaze-tests", None),
            completion("stack-the-shelves", None),
            completion("nothing left to do", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    // The small model that names a room. A pursuit dispatches into the
    // building, so every node needs a room of its own before it has one.
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: 32_768,
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"digest"),
        })
        .unwrap();
    worker
        .handle(channels::Command::Pursue {
            addr: Address::parse("lab").unwrap(),
            step: channels::PursuitStep::Set {
                goal: "fire the kiln".to_owned(),
            },
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"pursue"),
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let lines: Vec<serde_json::Value> = verified
        .raw_lines()
        .iter()
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    let started: std::collections::BTreeSet<String> = lines
        .iter()
        .take_while(|line| line["kind"] != "run_frozen")
        .filter(|line| line["kind"] == "run_started")
        .map(|line| line["run"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        started.len(),
        3,
        "three ready nodes are three runs going at once: {} had started when the first froze",
        started.len()
    );
    let frozen = lines
        .iter()
        .filter(|line| line["kind"] == "run_frozen")
        .count();
    assert_eq!(frozen, 3, "every run the pursuit started has to freeze");
}
