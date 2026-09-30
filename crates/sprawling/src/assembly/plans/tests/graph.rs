// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// A graph of nodes runs in dependency order, each in its own room
/// with its contract as its `JOB.md`, and what comes back verified
/// joins - so the next run in that room can be asked a question only
/// somebody who opened the results can answer.
///
/// The graph is laid out once. The node that waits is handed down when
/// what it waits on hands back, so no later run in the room has to lay
/// the same graph out again for the work to go on.
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
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "get it measured and written up".to_owned(),
            goal: "a page with a number in it, then stop".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    // Laying out a graph is a spawn like any other, and a spawn is
    // nobody's to approve: how deep work may be handed down is a type,
    // so the graph is laid out without anybody being asked.
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
        crate::assembly::fixture::node_lines(dir.path()).join("\n"),
        mentioned(&history, "run_frozen")
    );
}

/// **Card 3.5's property.** A pursuit takes the whole ready set: three
/// nodes nothing blocks are three runs going at once, not three runs one
/// after another.
///
/// The assertion is on the lanes rather than on a stopwatch: the
/// pursuit takes its rows off and returns, and nothing lands a run until
/// the loop below serves the crossing, so a sequential pursuit has one
/// run in the air at that moment and this one has three. The history
/// cannot say it: each run's preparation happens in its own lane, so how
/// its first line interleaves with another run's freeze is a race.
///
/// Each node's room is named by rule, so no reply is spent on a name;
/// the turns are concurrent and all get the same reply, which is what
/// keeps this test's provider script order-free where the order is not
/// this test's to decide.
#[test]
fn three_ready_nodes_drive_three_runs_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
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
    worker.serve_one(crate::assembly::Posted {
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

/// Every ledger line naming a word, joined for an assertion message.
///
/// A join count of one instead of two says a handback went missing and
/// nothing about which one, so the message carries the lines that would
/// have recorded it.
fn mentioned(history: &str, word: &str) -> String {
    let mut out: Vec<&str> = history.lines().filter(|line| line.contains(word)).collect();
    if out.is_empty() {
        out = vec!["(no line names it)"];
    }
    out.join("\n")
}

/// The ledger line around a byte offset, for an assertion that would
/// otherwise fail with two numbers and no page to look at.
///
/// The first mention of a node's address in the history is not always
/// the node's own record: a plan is laid out by naming every room in it,
/// and that naming is one line the history holds. An assertion that
/// compares two `find` offsets has to show the line it landed on, or the
/// next person re-derives which of the two it meant.
fn excerpt(history: &str, at: usize) -> String {
    // Byte offsets into a ledger line, which is ASCII JSON, so the
    // slicing clippy warns about cannot land inside a character here.
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
