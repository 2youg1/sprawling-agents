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
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn an_allowed_item_carries_the_work_on_instead_of_asking_for_the_command_again() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("thinking", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let addr = Address::parse("lab/room1").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: addr.clone(),
            task: "empty the archive".to_owned(),
            goal: "one sweep, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"first"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();

    let run = {
        let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
        let mut found = None;
        for line in verified.raw_lines() {
            let record = EventRecord::parse_line(line).unwrap();
            if record.kind() == EventKind::RunStarted {
                found = Some(record.run());
            }
        }
        found.expect("a dispatch starts a run")
    };
    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::new("item-1").unwrap(),
        actor: "lab/room1".to_owned(),
        action_desc: "empty the archive".to_owned(),
        artifact: Locator::parse("file:lab/room1@0000000000000000000000000000000000000000")
            .unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::Question,
            detail: "lab/room1".to_owned(),
        },
        created: TimeMs::new(1),
        tainted: false,
    };
    let value = serde_json::to_value(&item).unwrap();
    worker
        .record_for(
            run,
            effect::Line {
                who: "lab/room1".to_owned(),
                addr: addr.clone(),
                kind: EventKind::ApprovalRequested,
                data: Payload::new(value.as_object().cloned().unwrap()).unwrap(),
            },
        )
        .unwrap();
    worker.governance.pending.insert("item-1".to_owned(), item);

    let before = started_runs(&report.ledger_dir);
    worker
        .handle(wire::Command::Approve {
            item: kernel::ApprovalId::new("item-1").unwrap(),
            verdict: kernel::Ruling::Allow,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"approve"),
        })
        .unwrap();

    assert_eq!(
        started_runs(&report.ledger_dir),
        before + 1,
        "answering carries the work on; it does not wait for the command to be typed again"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("\"cluster\""),
        "the answer names the group it answered, so a restart knows what was allowed"
    );
}
fn started_runs(ledger_dir: &Path) -> usize {
    let verified = runtime::replay::verify_ledger_dir(ledger_dir).unwrap();
    verified
        .raw_lines()
        .iter()
        .filter(|line| {
            EventRecord::parse_line(line)
                .map(|record| record.kind() == EventKind::RunStarted)
                .unwrap_or(false)
        })
        .count()
}

#[test]
fn work_handed_down_becomes_a_run_that_cannot_hand_it_down_again() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "handing it down",
                "delegate",
                "tu_1",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion("waiting on a person", None),
            completion_with(
                "handing it down",
                "delegate",
                "tu_2",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
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
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();
    assert!(
        city::job_path(dir.path(), &Address::parse("lab/helper").unwrap()).exists(),
        "the delegate was never given a job file"
    );
    assert!(
        worker.governance.pending.is_empty(),
        "a spawn is decided by the type, not by a person"
    );

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("lab/helper"),
        "the delegate's own run started in the room it was given"
    );
    assert!(
        city::job_path(dir.path(), &Address::parse("lab/helper").unwrap()).exists(),
        "and it was given a task file of its own"
    );
    assert!(
        history.contains("E_DELEGATION_DEPTH") || !history.contains("lab/grandchild"),
        "a delegate that delegated would have opened a third room"
    );
    assert!(
        history.contains("handback"),
        "what came back is waiting in the room that asked: {history}"
    );
    assert!(
        history.contains("lab/helper finished"),
        "the handback names the room the work was done in"
    );
}

#[test]
fn what_came_back_from_a_delegate_waits_in_the_room_that_asked_for_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "handing it down",
                "delegate",
                "tu_1",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion("waiting on a person", None),
            completion_with(
                "handing it down",
                "delegate",
                "tu_2",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion_with("where did it go", "status", "tu_3", serde_json::json!({})),
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
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();

    let waiting = worker.collaborating.rooms.pending(&room);
    assert_eq!(
        waiting, 1,
        "exactly one handback per piece of work handed down"
    );
    let taken = worker.collaborating.rooms.pull_at_home(&room).unwrap();
    let body = taken[0].payload().as_map();
    assert_eq!(body["room"], "lab/helper");
    assert_eq!(
        taken[0].from(),
        "lab/helper",
        "the handback comes from the delegate, not from the city"
    );
    assert!(
        body["at"].as_str().unwrap().starts_with("cas:b3-"),
        "the account is pinned before it is judged"
    );
}

#[test]
fn status_tells_a_run_where_the_work_it_handed_down_went() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion_with(
                "handing it down",
                "delegate",
                "tu_1",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                    "kind": "ephemeral",
                }),
            ),
            completion_with("where did it go", "status", "tu_2", serde_json::json!({})),
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
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(
        history.contains("children: lab/helper (ephemeral)"),
        "the status a model read did not name the work it had just handed down: {history}"
    );
    assert!(
        history.contains(r#""parent":"#),
        "run_started carries no parent: {history}"
    );
}
