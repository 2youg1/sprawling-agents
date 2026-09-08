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
