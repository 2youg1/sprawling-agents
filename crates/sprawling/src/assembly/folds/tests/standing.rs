// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one verified pass rebuilds: the queues and the ground a worker
//! inherits, the endpoint book a probe filled, and the scopes a person shut.

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;
use kernel::Locator;

/// What a working worker holds and what a restarted one rebuilds are
/// one thing folded by one rule, or they are two things that happen
/// to agree. This asks which.
#[test]
fn what_a_worker_holds_is_what_a_restart_rebuilds() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("market").join("ito")).unwrap();
    std::fs::create_dir_all(dir.path().join("market").join("hana")).unwrap();
    lay_rules(
        dir.path(),
        "market",
        "# BUILDING.md\n\n`confidential: false`\n",
    );

    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "asking hana",
                "tu_1",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/hana",
                    "text": "what is your rate?",
                }),
            ),
            completion("asked", None),
            completion("nothing to add", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "ask hana what she charges".to_owned(),
            goal: "a price".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let rebuilt = Standing::fold(&report.ledger_dir).unwrap().collaboration;
    let live_queues: std::collections::BTreeMap<String, u32> = worker
        .inboxes
        .iter()
        .filter(|(_, queue)| queue.pending() > 0)
        .map(|(room, queue)| (room.as_str().to_owned(), queue.pending()))
        .collect();
    let rebuilt_queues: std::collections::BTreeMap<String, u32> = rebuilt
        .inboxes
        .iter()
        .filter(|(_, queue)| queue.pending() > 0)
        .map(|(room, queue)| (room.as_str().to_owned(), queue.pending()))
        .collect();
    assert_eq!(
        live_queues, rebuilt_queues,
        "a queue the working city holds is a queue a restart finds"
    );
    assert_eq!(
        worker.goals.len(),
        rebuilt.goals.len(),
        "the ground claimed is folded from one rule"
    );
    assert_eq!(
        worker.requests.len(),
        rebuilt.requests.len(),
        "the register of open requests is folded from one rule"
    );

    // What each waiting item is holding up. This is the half the
    // handoff left open: the origin is process memory, and a
    // restarted worker rebuilds from the ledger, so an item raised
    // before a restart and answered after it has to find the same
    // work. It does, because the origin is folded by `absorb` out of
    // the same two lines a restart reads.
    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::new("item-held").unwrap(),
        source: kernel::ApprovalSource::Gate,
        actor: "market/ito".to_owned(),
        action_desc: "ask hana what she charges".to_owned(),
        artifact: Locator::parse("file:market/ito@0000000000000000000000000000000000000000")
            .unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::DiscardEscalate,
            detail: "market/ito".to_owned(),
        },
        created: TimeMs::new(1),
        tainted: false,
    };
    let started = {
        let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
        let mut first = None;
        for line in verified.raw_lines() {
            let record = EventRecord::parse_line(line).unwrap();
            if record.kind() == EventKind::RunStarted && first.is_none() {
                first = Some(record.run());
            }
        }
        first.expect("a dispatch starts a run")
    };
    worker
        .record_for(
            started,
            effect::Line {
                who: "market/ito".to_owned(),
                addr: Address::parse("market/ito").unwrap(),
                kind: EventKind::ApprovalRequested,
                data: Payload::new(
                    serde_json::to_value(&item)
                        .unwrap()
                        .as_object()
                        .cloned()
                        .unwrap(),
                )
                .unwrap(),
            },
        )
        .unwrap();
    let refolded = Standing::fold(&report.ledger_dir).unwrap().governance;
    assert_eq!(
        worker.governance.origins, refolded.origins,
        "what a waiting item is holding up is folded from one rule, so an answer after a \
         restart resumes the work an answer before it would have"
    );
    assert_eq!(
        worker.governance.origins.get("item-held"),
        Some(&BlockedJob {
            addr: Address::parse("market/ito").unwrap(),
            task: "ask hana what she charges".to_owned(),
            goal: "a price".to_owned(),
        }),
        "and the comparison above is not two empty maps agreeing"
    );
}

/// A person could not see what a key bought until they had already
/// registered it, and could not register part of what it bought.
#[test]
fn a_provider_can_be_asked_what_it_serves_and_only_part_of_it_admitted() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-small", "m-large"], Vec::new());
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    worker
        .handle(channels::Command::ProbeEndpoint {
            name: channels::ProviderName::parse("house").unwrap(),
            base_url: base_url.clone(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            tuning: channels::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"probe"),
        })
        .unwrap();
    let after_probe = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let probed: String = after_probe
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(probed.contains("endpoint_probed"), "{probed}");
    assert!(probed.contains("m-small") && probed.contains("m-large"));
    assert!(
        !probed.contains("endpoint_attached"),
        "asking what a provider serves attached it anyway"
    );

    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["m-large".to_owned()],
            tuning: channels::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    let channels::Answer::Endpoints(book) = rebuild_views(&report.ledger_dir)
        .unwrap()
        .answer(&channels::Query::EndpointView)
    else {
        panic!("the settings page reads the endpoint book");
    };
    assert_eq!(
        book.endpoints[0].models,
        vec!["m-large".to_owned()],
        "a subset was ticked and the whole list was registered anyway"
    );
}

/// Halting is admission control: it refuses new work and says which
/// scope refused, and a release opens the same scope again. Both
/// survive a restart, because the worker folds them from the ledger
/// rather than holding them only in memory.
#[test]
fn a_halted_scope_refuses_new_work_and_a_release_takes_it_again() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let work = |tag: &[u8]| channels::Command::Dispatch {
        addr: room.clone(),
        task: "measure the thing".to_owned(),
        goal: "a number, then stop".to_owned(),
        mode: channels::ModeTag::parse("plan").unwrap(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, tag),
        session: None,
        effort: None,
    };
    let halt = |scope| channels::Command::Halt {
        scope,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"halt"),
    };

    worker
        .handle(halt(channels::HaltScope::Building(
            Address::parse("lab").unwrap(),
        )))
        .unwrap();
    let refused = worker.handle(work(b"one")).unwrap_err();
    assert_eq!(refused.code(), &AxCode::GateDenied);
    assert!(
        refused.recovery().contains("release"),
        "a refusal says how to undo the thing that caused it: {}",
        refused.recovery()
    );
    assert!(
        !city::job_path(dir.path(), &room).exists(),
        "a refused dispatch leaves no task in a room no run opened"
    );

    // A different building is not covered by that halt.
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("shop").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let mut elsewhere = work(b"two");
    if let channels::Command::Dispatch { addr, .. } = &mut elsewhere {
        *addr = Address::parse("shop/room1").unwrap();
    }
    worker.handle(elsewhere).unwrap();

    worker
        .handle(channels::Command::Release {
            scope: channels::HaltScope::Building(Address::parse("lab").unwrap()),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"release"),
        })
        .unwrap();
    worker.handle(work(b"three")).unwrap();

    // The posture is history, not a field: a second worker over the
    // same ledger knows the building is open again.
    let restarted = Standing::fold(&ledger_dir(dir.path())).unwrap().governance;
    assert!(restarted.halted.is_empty());
}
