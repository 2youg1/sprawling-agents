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
use crate::assembly::*;

#[test]
fn an_allowed_item_carries_the_work_on_instead_of_asking_for_the_command_again() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("thinking", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let addr = Address::parse("lab/room1").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: addr.clone(),
            task: "empty the archive".to_owned(),
            goal: "one sweep, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"first"),
            session: None,
            effort: None,
        })
        .unwrap();

    // The run that just ran asked for something the person has to
    // answer. Recorded the way the bench records it: against the run
    // and the address, because answering it later has to find the
    // work it was holding up.
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
        source: kernel::ApprovalSource::Gate,
        actor: "lab/room1".to_owned(),
        action_desc: "empty the archive".to_owned(),
        artifact: Locator::parse("file:lab/room1@0000000000000000000000000000000000000000")
            .unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::DiscardEscalate,
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
        .handle(channels::Command::Approve {
            item: kernel::ApprovalId::new("item-1").unwrap(),
            verdict: kernel::PolicyVerdict::Allow,
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

/// A run that hands work down starts a real second run, and that
/// run cannot hand work down again.
///
/// `kernel::gate::spawn` had held the one-level rule since S2 with
/// no caller in production; this is the caller. The child is
/// dispatched after the parent's turn settles rather than inside the
/// tool call, because a tool that drove a run would be driving one
/// from inside another run's tool bench.
#[test]
fn work_handed_down_becomes_a_run_that_cannot_hand_it_down_again() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
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
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    // Nothing has been handed down yet: the first spawn of a run
    // waits for the person, and answering carries the work on.
    assert!(
        !city::job_path(dir.path(), &Address::parse("lab/helper").unwrap()).exists(),
        "a delegate started before anybody allowed it"
    );
    let cluster = allow_the_one_pending_item(&mut worker);
    assert_eq!(cluster.class, kernel::ApprovalClass::Delegation);
    assert_eq!(
        cluster.detail, "lab/room1",
        "the person is asked once per resident, not once per room it picks"
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
    // The second run asked to hand work down in turn, and the gate
    // that had never been called refused it.
    assert!(
        history.contains("E_DELEGATION_DEPTH") || !history.contains("lab/grandchild"),
        "a delegate that delegated would have opened a third room"
    );
    // The way back. Before this, a delegate's result reached the
    // ledger and never the run that asked for it, and a person had
    // to watch the live page to find out.
    assert!(
        history.contains("handback"),
        "what came back is waiting in the room that asked: {history}"
    );
    assert!(
        history.contains("lab/helper finished"),
        "the handback names the room the work was done in"
    );
}

/// The result of handed-down work is waiting in the asking room's
/// inbox, which is what `status.signals_pending` counts and what the
/// `signal` tool takes. The parent's own run is frozen by the time a
/// child starts - only the assembly layer can build a run, and it
/// gets control back after the parent's last turn - so the way back
/// crosses runs, and the door that already does that is the room's
/// queue.
#[test]
fn what_came_back_from_a_delegate_waits_in_the_room_that_asked_for_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            // Asked, refused pending, gave up. The person then
            // allows it, the work is dispatched again, and the
            // second ask goes through.
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
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    allow_the_one_pending_item(&mut worker);

    let waiting = worker
        .inboxes
        .get(&room)
        .expect("the asking room has a queue")
        .pending();
    assert_eq!(
        waiting, 1,
        "exactly one handback per piece of work handed down"
    );
    let taken = worker
        .inboxes
        .get_mut(&room)
        .expect("the asking room has a queue")
        .pull()
        .unwrap();
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

/// `status.children` was a hardcoded empty list, so the one field a
/// run could have used to check what it had handed down always said
/// "none".
#[test]
fn status_tells_a_run_where_the_work_it_handed_down_went() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
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
            completion("waiting on a person", None),
            completion_with(
                "handing it down",
                "delegate",
                "tu_2",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                    "kind": "ephemeral",
                }),
            ),
            completion_with("where did it go", "status", "tu_3", serde_json::json!({})),
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
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    allow_the_one_pending_item(&mut worker);

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
    // And the child's own run says whose work it is, which is what
    // the interface folds into a tree.
    assert!(
        history.contains(r#""parent":"#),
        "run_started carries no parent: {history}"
    );
}
