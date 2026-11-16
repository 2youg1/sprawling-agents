// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use crate::worker::fixture::*;
use kernel::{Address, EventKind, EventRecord, RunId};

fn tool_tables(bodies: &[String]) -> Vec<Vec<String>> {
    bodies
        .iter()
        .filter_map(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .filter_map(|value| {
            let tools = value["tools"].as_array()?.clone();
            if tools.is_empty() {
                return None;
            }
            tools
                .iter()
                .map(|tool| tool["function"]["name"].as_str().map(str::to_owned))
                .collect()
        })
        .collect()
}

#[test]
fn three_successions_keep_the_tools_and_leave_a_lineage_of_four() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    let ask = |id: &str| {
        tool_completion(
            "handing over",
            id,
            "succeed",
            serde_json::json!({ "reason": "the window is filling" }),
        )
    };

    let probed = || completion("1. the lexer\n2. nothing yet\n3. no", None);
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            ask("tu_1"),
            completion("over to you", None),
            probed(),
            probed(),
            ask("tu_2"),
            completion("over to you", None),
            probed(),
            probed(),
            ask("tu_3"),
            completion("over to you", None),
            probed(),
            probed(),
            tool_completion(
                "noting",
                "tu_4",
                "edit",
                serde_json::json!({
                    "file_path": "lab/room1/notes.md",
                    "old_string": "",
                    "new_string": "the lexer\n"
                }),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "carry a long piece of work".to_owned(),
            goal: "it ends".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"succession"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let bodies = provider.bodies();
    drop(provider);

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let records: Vec<EventRecord> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .collect();
    let started: Vec<&EventRecord> = records
        .iter()
        .filter(|record| record.kind() == EventKind::RunStarted)
        .collect();
    assert_eq!(started.len(), 4, "three successions are four runs");
    for pair in started.windows(2) {
        let predecessor = pair[1].data().as_map()["predecessor"]
            .as_str()
            .expect("a successor's run_started names its predecessor");
        assert_eq!(predecessor, pair[0].run().to_string());
        assert_eq!(
            pair[1].addr(),
            pair[0].addr(),
            "a successor works at the same address"
        );
        assert!(
            !pair[1].data().as_map().contains_key("parent"),
            "succession is not delegation: depth is conserved"
        );
    }

    let tables = tool_tables(&bodies);
    let first = tables.first().expect("the first run called the model");
    let last = tables.last().expect("the fourth run called the model");
    assert!(first.contains(&"tool".to_owned()), "{first:?}");

    let own = |body: &str| {
        body.split("- sprawling (")
            .nth(1)
            .and_then(|rest| rest.split_once(')'))
            .map(|(count, _)| count.to_owned())
    };
    let counted = bodies
        .iter()
        .find_map(|body| own(body))
        .expect("a request carried an index line");
    let indexed = bodies
        .iter()
        .filter(|body| own(body).as_deref() == Some(counted.as_str()))
        .count();
    assert!(indexed >= 4, "{indexed} requests filed {counted} own tools");
    assert_eq!(
        first, last,
        "the successor's tools equal its predecessor's, name for name"
    );

    let checkpointed = records
        .iter()
        .find(|record| {
            record.kind() == EventKind::CheckpointCommitted
                && record.data().as_map().contains_key("oid")
                && record.run() == started[3].run()
        })
        .expect("the fourth run's wave checkpoints a commit");
    let oid = kernel::GitOid::parse(checkpointed.data().as_map()["oid"].as_str().unwrap()).unwrap();
    let views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();
    let wire::Answer::Commit(said) = views.prepare(&wire::Query::Commit { oid }).finish() else {
        panic!("a commit this city made answers which run wrote it");
    };
    let expected: Vec<RunId> = started.iter().rev().map(|record| record.run()).collect();
    assert_eq!(
        said.lineage, expected,
        "this run first, then each predecessor"
    );
}

#[test]
fn the_handoff_in_the_room_reaches_the_successor() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let room = dir.path().join("lab").join("room1");
    std::fs::create_dir_all(&room).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "writing the handoff",
                "tu_1",
                "edit",
                serde_json::json!({
                    "file_path": "lab/room1/Handoff.md",
                    "old_string": "",
                    "new_string": "# Handoff\n\nThe parser is half done; resume at the lexer.\n"
                }),
            ),
            tool_completion(
                "handing over",
                "tu_2",
                "succeed",
                serde_json::json!({ "reason": "enough for one run" }),
            ),
            completion("over to you", None),
            completion("picked up", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "write the parser".to_owned(),
            goal: "it parses".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"room-handoff"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let bodies = provider.bodies();
    drop(provider);
    assert!(
        room.join("Handoff.md").is_file(),
        "the handoff is the room's, not the building's"
    );
    assert!(
        !dir.path().join("lab").join("Handoff.md").is_file(),
        "the building keeps no handoff of its own"
    );
    let successor = bodies.last().unwrap();
    assert!(
        successor.contains("resume at the lexer"),
        "what the predecessor wrote reaches the successor's prompt"
    );
    assert!(
        successor.contains("Predecessor transcript: lab/room1/"),
        "the successor is told where its predecessor's conversation is: {successor}"
    );
}
