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

use crate::views::Views;
use crate::worker::*;
use crate::worker::{CommandDesk, Posted};
use kernel::Locator;

#[test]
fn an_outside_editor_asks_for_work_and_a_stranger_learns_one_bit() {
    let desk = CommandDesk::default();

    let body = |addr: &str| {
        serde_json::json!({
            "token": "pair-me",
            "addr": addr,
            "task": "read the plan",
            "goal": "one answer",
        })
    };

    let err = acp_dispatch(&desk, &body("lab/room1"), wire::Pairing::Absent).unwrap_err();
    assert_eq!(err.code(), &AxCode::GateDenied);
    assert!(
        !err.subject().contains("lab"),
        "an unauthenticated caller learns one bit and not whether the address exists: {}",
        err.subject()
    );
    assert!(desk.take().is_none(), "and nothing was queued for it");

    let err = acp_dispatch(&desk, &body(".sprawling/ledger"), wire::Pairing::Held).unwrap_err();
    assert_eq!(err.code(), &AxCode::OutsideWriteDomain);
    assert!(desk.take().is_none());

    let progress = acp_dispatch(&desk, &body("lab/room1"), wire::Pairing::Held).unwrap();
    assert!(!progress.finished);
    assert_eq!(progress.turns, 0);
    let Some(wire::Command::Dispatch { addr, task, .. }) = desk.take() else {
        panic!("an admitted request becomes the dispatch a person would have sent");
    };
    assert_eq!(addr.as_str(), "lab/room1");
    assert_eq!(task, "read the plan");
}

#[test]
fn a_refused_command_reaches_the_peer_that_sent_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();

    let heard: Arc<std::sync::Mutex<Vec<AxError>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let peer = Arc::clone(&heard);
    let reply = wire::Reply::to(move |error| {
        let Ok(mut heard) = peer.lock() else {
            return wire::Delivered::PeerGone;
        };
        heard.push(error);
        wire::Delivered::ToThePeer
    });

    worker.serve_one(Posted {
        command: wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("nowhere").unwrap(),
            model: "a-model".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(200_000),
            max_output_tokens: kernel::Ceiling::new(8_192),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        },
        reply,
    });

    let heard = heard.lock().unwrap();
    assert_eq!(heard.len(), 1, "the peer is told once, and told at all");
    let told = heard.first().unwrap();
    assert!(
        !told.recovery().is_empty(),
        "a refusal that reaches a person carries the way out: {told}"
    );
}

#[test]
fn a_refusal_with_no_one_behind_it_says_so_rather_than_failing() {
    let nobody = wire::Reply::nowhere();
    let outcome = nobody.refuse(
        AxError::failure(
            AxCode::ConfigInvalid,
            "read the schedule",
            "the file is not a schedule",
        )
        .with_recovery("fix the schedule file, then start the city again"),
    );
    assert_eq!(outcome, wire::Delivered::NobodyAsked);
}
#[test]
fn an_answer_lands_in_the_history_and_a_delegate_cannot_answer_its_own_action() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();

    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::new("item-1").unwrap(),
        actor: "lab/room1".to_owned(),
        action_desc: "send the release mail".to_owned(),
        artifact: Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::Question,
            detail: "mail:release".to_owned(),
        },
        created: kernel::TimeMs::new(1_000),
        tainted: false,
    };
    let payload = Payload::new(
        serde_json::to_value(&item)
            .unwrap()
            .as_object()
            .cloned()
            .unwrap(),
    )
    .unwrap();
    worker
        .record(EventKind::ApprovalRequested, payload)
        .unwrap();

    worker
        .handle(wire::Command::SetAutonomy {
            scope: wire::HaltScope::City,
            autonomy: kernel::Autonomy::Delegate(kernel::ResidentId::new("lab/room1").unwrap()),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"autonomy"),
        })
        .unwrap();
    let err = worker
        .answer_approval(
            &kernel::ApprovalId::new("item-1").unwrap(),
            kernel::Ruling::Allow,
            &kernel::Answerer::Resident(kernel::ResidentId::new("lab/room1").unwrap()),
        )
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::ApprovalDenied);
    assert!(err.subject().contains("SelfApprovalBarred"));

    worker
        .handle(wire::Command::Approve {
            item: kernel::ApprovalId::new("item-1").unwrap(),
            verdict: kernel::Ruling::Allow,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"approve"),
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("autonomy_changed"));
    assert!(history.contains("approval_resolved"));

    let err = worker
        .handle(wire::Command::Approve {
            item: kernel::ApprovalId::new("item-1").unwrap(),
            verdict: kernel::Ruling::Allow,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"approve"),
        })
        .unwrap_err();
    assert!(err.recovery().contains("not waiting"));

    drop(worker);
    let restarted = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    assert!(restarted.governance.pending.is_empty());
    assert_eq!(
        restarted.governance.autonomy,
        kernel::Autonomy::Delegate(kernel::ResidentId::new("lab/room1").unwrap())
    );
}

#[test]
fn the_approval_queue_holds_what_was_asked_and_drops_what_was_answered() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());

    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::new("a-1".to_owned()).unwrap(),
        actor: "urbanite-2".to_owned(),
        action_desc: "delete the archive".to_owned(),
        artifact: kernel::Locator::parse("file:lab/room1@0000000000000000000000000000000000000000")
            .unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::Question,
            detail: "lab".to_owned(),
        },
        created: TimeMs::new(1),
        tainted: true,
    };
    let asked = serde_json::to_value(&item)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    let requested = EventRecord::from_draft(
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(1),
            who: "gate".to_owned(),
            addr: None,
            kind: EventKind::ApprovalRequested,
            data: Payload::new(asked).unwrap(),
            ig: false,
        },
        kernel::Seq::FIRST,
        kernel::GENESIS_PREV,
    );
    views.apply(&requested).unwrap();
    let wire::Answer::Approvals(queue) = views.prepare(&wire::Query::ApprovalQueue).finish() else {
        panic!("the approval queue answers with items");
    };
    assert_eq!(queue.items.len(), 1);
    assert!(queue.items[0].tainted);
    assert_eq!(
        queue.items[0], item,
        "what the queue answers is the item the ledger recorded, field for field"
    );
    assert_eq!(
        queue.items[0].cluster_key.detail, "lab",
        "the cluster key survives, or the inbox cannot group anything"
    );

    let mut answered = serde_json::Map::new();
    answered.insert("id".to_owned(), serde_json::Value::String("a-1".to_owned()));
    answered.insert(
        "verdict".to_owned(),
        serde_json::Value::String("allow".to_owned()),
    );
    answered.insert(
        "cluster".to_owned(),
        serde_json::json!({ "class": "question", "detail": "lab" }),
    );
    let resolved = EventRecord::from_draft(
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(2),
            who: "owner".to_owned(),
            addr: None,
            kind: EventKind::ApprovalResolved,
            data: Payload::new(answered).unwrap(),
            ig: false,
        },
        kernel::Seq::FIRST.next().unwrap(),
        kernel::GENESIS_PREV,
    );
    views.apply(&resolved).unwrap();
    let wire::Answer::Approvals(queue) = views.prepare(&wire::Query::ApprovalQueue).finish() else {
        panic!("the approval queue answers with items");
    };
    assert!(queue.items.is_empty());
}
#[test]
fn a_command_with_no_executor_is_refused_by_name_and_not_by_stage() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();

    let missing = worker
        .handle(wire::Command::Cancel {
            run: RunId::CITY,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"cancel"),
        })
        .unwrap_err();
    assert_eq!(*missing.code(), AxCode::InvalidArgs);
    assert!(
        missing.recovery().contains("no run in flight"),
        "the refusal names the run, not the build stage: {}",
        missing.recovery()
    );
    assert!(!missing.recovery().contains("cards"));

    let unbuilt = worker
        .handle(wire::Command::BatchByBuilding {
            addr: Address::parse("lab").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"batch"),
        })
        .unwrap_err();
    assert_eq!(*unbuilt.code(), AxCode::WireMismatch);
    assert!(
        unbuilt.recovery().contains("not built"),
        "an unbuilt verb says so: {}",
        unbuilt.recovery()
    );
    assert!(
        unbuilt
            .recovery()
            .contains("dispatch the rooms one at a time"),
        "and names what to do instead: {}",
        unbuilt.recovery()
    );
}
