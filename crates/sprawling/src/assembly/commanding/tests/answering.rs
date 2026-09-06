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

use crate::assembly::*;
use crate::serving::CommandDesk;
use crate::views::Views;

#[test]
fn an_outside_editor_asks_for_work_and_a_stranger_learns_one_bit() {
    let desk = CommandDesk::new();
    let body = |addr: &str| channels::AcpBody {
        token: "pair-me".to_owned(),
        addr: addr.to_owned(),
        task: "read the plan".to_owned(),
        goal: "one answer".to_owned(),
    };

    let err = acp_dispatch(&desk, body("lab/room1"), false).unwrap_err();
    assert_eq!(err.code(), &AxCode::GateDenied);
    assert!(
        !err.subject().contains("lab"),
        "an unauthenticated caller learns one bit and not whether the address exists: {}",
        err.subject()
    );
    assert!(desk.take().is_none(), "and nothing was queued for it");

    // The city's own subtree is not a room, with or without a token.
    let err = acp_dispatch(&desk, body(".sprawling/ledger"), true).unwrap_err();
    assert_eq!(err.code(), &AxCode::OutsideWriteDomain);
    assert!(desk.take().is_none());

    let progress = acp_dispatch(&desk, body("lab/room1"), true).unwrap();
    assert!(!progress.finished);
    assert_eq!(progress.turns, 0);
    let Some(channels::Command::Dispatch { addr, task, .. }) = desk.take() else {
        panic!("an admitted request becomes the dispatch a person would have sent");
    };
    assert_eq!(addr.as_str(), "lab/room1");
    assert_eq!(task, "read the plan");
}

/// The defect this card exists for: a person presses a button, the
/// city refuses, and the page says nothing at all. The refusal has
/// to arrive at whoever caused it - found by driving a real city
/// over its own wire, where two refused commands appeared in the
/// server's log file and nowhere else.
#[test]
fn a_refused_command_reaches_the_peer_that_sent_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    let heard: Arc<std::sync::Mutex<Vec<AxError>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let peer = Arc::clone(&heard);
    let reply = channels::Reply::to(move |error| {
        let Ok(mut heard) = peer.lock() else {
            return channels::Delivered::PeerGone;
        };
        heard.push(error);
        channels::Delivered::ToThePeer
    });

    // A model chosen on an endpoint that was never attached: the
    // same shape as the real failure, where a base URL missing its
    // `/v1` made the attach fail and the model selection fail after
    // it, and the page reported neither.
    worker.serve_one(Posted {
        command: channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("nowhere").unwrap(),
            model: "a-model".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: 200_000,
            max_output_tokens: 8_192,
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

/// A command nobody sent - the schedule's own - is refused into a
/// reply address that names the absence, rather than into a peer
/// that would have to be invented for it.
#[test]
fn a_refusal_with_no_one_behind_it_says_so_rather_than_failing() {
    let nobody = channels::Reply::nowhere();
    let outcome = nobody.refuse(AxError::failure(
        AxCode::ConfigInvalid,
        "read the schedule",
        "the file is not a schedule",
    ));
    assert_eq!(outcome, channels::Delivered::NobodyAsked);
}
#[test]
fn an_answer_lands_in_the_history_and_a_delegate_cannot_answer_its_own_action() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::new("item-1").unwrap(),
        source: kernel::ApprovalSource::Agent,
        actor: "lab/room1".to_owned(),
        action_desc: "send the release mail".to_owned(),
        artifact: Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::AgentQuestion,
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

    // The appointed delegate is the actor of this item, so the one
    // resident allowed to answer at all is barred from this one.
    worker
        .handle(channels::Command::SetAutonomy {
            scope: channels::HaltScope::City,
            autonomy: kernel::Autonomy::Delegate(kernel::ResidentId::new("lab/room1").unwrap()),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"autonomy"),
        })
        .unwrap();
    let err = worker
        .answer_approval(
            &kernel::ApprovalId::new("item-1").unwrap(),
            kernel::PolicyVerdict::Allow,
            &kernel::Answerer::Resident(kernel::ResidentId::new("lab/room1").unwrap()),
        )
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::ApprovalDenied);
    assert!(err.subject().contains("SelfApprovalBarred"));

    // The person answers it, and the history says so.
    worker
        .handle(channels::Command::Approve {
            item: kernel::ApprovalId::new("item-1").unwrap(),
            verdict: kernel::PolicyVerdict::Allow,
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

    // And a second answer finds nothing waiting: the queue drains
    // from the ledger, not from a mirror of it.
    let err = worker
        .handle(channels::Command::Approve {
            item: kernel::ApprovalId::new("item-1").unwrap(),
            verdict: kernel::PolicyVerdict::Allow,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"approve"),
        })
        .unwrap_err();
    assert!(err.recovery().contains("not waiting"));

    // A worker that restarts reads the same answer out of the ledger.
    let restarted = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    assert!(restarted.governance.pending.is_empty());
    assert_eq!(
        restarted.governance.autonomy,
        kernel::Autonomy::Delegate(kernel::ResidentId::new("lab/room1").unwrap())
    );
}

/// The wire promises that "double-clicking twice opens two Runs" is
/// not reachable, and every client mints its key from what the
/// person entered, so the same request twice is the same key twice.
/// Nothing read that key until this desk did.
#[test]
fn the_approval_queue_holds_what_was_asked_and_drops_what_was_answered() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    // The payload is built the way the dispatch path builds it -
    // by serialising the item - rather than out of field names a
    // test invented. The invented shape was the defect: the view
    // read a `summary` key nobody ever wrote, so every waiting item
    // rendered as "(no summary recorded)" and no test noticed.
    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::new("a-1".to_owned()).unwrap(),
        source: kernel::ApprovalSource::Gate,
        actor: "urbanite-2".to_owned(),
        action_desc: "delete the archive".to_owned(),
        artifact: kernel::Locator::parse("file:lab/room1@0000000000000000000000000000000000000000")
            .unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::AgentQuestion,
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
    let channels::Answer::Approvals(queue) = views.answer(&channels::Query::ApprovalQueue) else {
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
    let channels::Answer::Approvals(queue) = views.answer(&channels::Query::ApprovalQueue) else {
        panic!("the approval queue answers with items");
    };
    assert!(queue.items.is_empty());
}
#[test]
fn a_command_with_no_executor_is_refused_by_name_and_not_by_stage() {
    // What this replaced: one catch-all whose recovery read "this
    // stage runs Dispatch; the rest land with their cards", which is
    // a sentence about a build stage that ended. Eight commands
    // reached it, three of them from buttons this client used to
    // draw, and a person pressing one learned nothing they could act
    // on. The match is exhaustive now, so a command added without an
    // executor stops the build rather than reaching a person.
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    // Cancel has a second door - the desk lifts it off the queue at
    // the next safe point of the run it names - so arriving here
    // means no run answered, and the refusal says that rather than
    // naming the verb.
    let missing = worker
        .handle(channels::Command::Cancel {
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

    // A verb the wire spells and this city cannot perform. It says
    // so, and says what to do instead.
    let unbuilt = worker
        .handle(channels::Command::Takeover {
            run: RunId::CITY,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"takeover"),
        })
        .unwrap_err();
    assert_eq!(*unbuilt.code(), AxCode::WireMismatch);
    assert!(
        unbuilt.recovery().contains("not built"),
        "an unbuilt verb says so: {}",
        unbuilt.recovery()
    );
    assert!(
        unbuilt.recovery().contains("Steer") || unbuilt.recovery().contains("steer"),
        "and names what to do instead: {}",
        unbuilt.recovery()
    );
}
