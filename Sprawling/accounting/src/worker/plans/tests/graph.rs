// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::worker::fixture::*;

#[test]
fn a_workshop_runs_its_nodes_in_order_and_what_comes_back_joins() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
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
    let (base_url, _provider) = fake_openai_routed(
        &["m-local"],
        vec![
            ("a number is written down", vec![completion("done", None)]),
            ("the page exists", vec![completion("done", None)]),
        ],
        vec![
            completion_with("splitting it up", "workshop", "tu_1", graph),
            completion("waiting on a person", None),
            completion("not laying it out again", None),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "get it measured and written up".to_owned(),
            goal: "a page with a number in it, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();

    assert!(
        worker.governance.pending.is_empty(),
        "a spawn is decided by the type, not by a person"
    );

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
        "the node everything waits on has to go first
         read {reader}, wrote {writer}
         the earlier mention sits in:
{}",
        excerpt(&history, reader.min(writer))
    );
    assert_eq!(
        worker
            .collaborating
            .joins
            .get(&room)
            .map_or(0, |join| join.artifacts().count()),
        2,
        "both results joined, verified by the city rather than by their own producers\n\
         the nodes wrote:\n{}\n\
         and how each run ended:\n{}",
        crate::worker::fixture::node_lines(dir.path()).join("\n"),
        mentioned(&history, "run_frozen")
    );
}

#[test]
fn three_ready_nodes_drive_three_runs_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    lay_rules(dir.path(), "lab", &ordinary_rules(""));
    std::fs::write(
        dir.path().join("lab").join(city::ROADMAP_FILE),
        PLAN_THREE_FREE_ROWS,
    )
    .unwrap();
    let (base_url, _provider) =
        fake_openai(&["m-local"], vec![completion("nothing left to do", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker.serve_one(crate::worker::Posted {
        command: wire::Command::Pursue {
            addr: Address::parse("lab").unwrap(),
            step: wire::PursuitStep::Set {
                goal: "fire the kiln".to_owned(),
            },
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"pursue"),
        },
        reply: wire::Reply::nowhere(),
    });
    let in_the_air = worker.flight.in_flight();
    worker.land_the_rest().unwrap();

    assert_eq!(
        in_the_air, 3,
        "three ready nodes are three runs going at once"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let lines: Vec<serde_json::Value> = verified
        .raw_lines()
        .iter()
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    let frozen = lines
        .iter()
        .filter(|line| line["kind"] == "run_frozen")
        .count();
    assert_eq!(frozen, 3, "every run the pursuit started has to freeze");
}

fn mentioned(history: &str, word: &str) -> String {
    let mut out: Vec<&str> = history.lines().filter(|line| line.contains(word)).collect();
    if out.is_empty() {
        out = vec!["(no line names it)"];
    }
    out.join("\n")
}

fn excerpt(history: &str, at: usize) -> String {
    let before = history.get(..at).unwrap_or_default();
    let start = before
        .rfind(char::from(10))
        .map_or(0, |line| line.saturating_add(1));
    let after = history.get(at..).unwrap_or_default();
    let end = after
        .find(char::from(10))
        .map_or(history.len(), |line| at.saturating_add(line));
    history.get(start..end).unwrap_or_default().to_owned()
}
