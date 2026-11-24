// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::collections::BTreeSet;
use std::sync::{Arc, Condvar, Mutex};

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::waking::handing::{GraphAfter, Handing};

const PARENT: &str = "lab/room1";

fn diamond() -> Vec<collab::NodeContract> {
    [
        ("n1", &[][..]),
        ("n2", &["n1"][..]),
        ("n3", &["n1"][..]),
        ("n4", &["n2", "n3"][..]),
    ]
    .into_iter()
    .map(|(id, deps)| {
        collab::NodeContract::new(
            collab::NodeId::parse(&format!("lab/{id}")).unwrap(),
            format!("produce {id}"),
            deps.iter()
                .map(|dep| collab::NodeId::parse(&format!("lab/{dep}")).unwrap())
                .collect::<BTreeSet<_>>(),
            Vec::new(),
            Address::parse(&format!("lab/{id}")).unwrap(),
            PARENT.to_owned(),
            format!("{id} is checked"),
            format!("stop when {id} is checked"),
        )
        .unwrap()
    })
    .collect()
}

fn laid_out(
    dir: &std::path::Path,
    stopping: &[&str],
    pace: Pace,
) -> (RunWorker, std::path::PathBuf, Box<dyn std::any::Any>) {
    let report = crate::worker::fixture::init_city(dir).unwrap();
    let routes = ["n1", "n2", "n3", "n4"].map(|id| {
        let reply = if stopping.contains(&id) { "" } else { "done" };
        (format!("{id} is checked"), vec![completion(reply, None)])
    });
    let (base_url, provider) = fake_openai_paced(
        &["m-local"],
        routes
            .iter()
            .map(|(key, replies)| (key.as_str(), replies.clone()))
            .collect(),
        vec![completion("done", None)],
        pace,
    );
    let mut worker = worker_with_provider(dir, &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            source: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let parent = Address::parse(PARENT).unwrap();
    let mut delegates = collab::DelegateDesk::new(
        kernel::Depth::Root,
        Address::parse("lab").unwrap(),
        collab::StartCheck::new(|_room: &Address| Ok(())),
    );
    let mut workshop =
        collab::WorkshopDesk::new(PARENT.to_owned(), collab::FanIn::new(), BTreeSet::new());
    workshop.lay_out(diamond(), &mut delegates).unwrap();
    let at = Assignment {
        addr: parent,
        session: None,
        resume: None,
        pictures: Vec::new(),
        effort: None,
        difficulty: None,
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        origin: None,
        parent: None,
        succession: None,
        taint: kernel::TaintSet::empty(),
        dispatched_by: kernel::event::Who::City,
    };
    let handing = Handing::new(
        &at,
        Arc::new(Mutex::new(delegates)),
        Arc::new(Mutex::new(workshop)),
        &Owing::unasked(Unasked::Schedule),
    );
    worker.collaborating.handing.insert(RunId::CITY, handing);
    worker.hand_over_at_call(RunId::CITY).unwrap();
    (worker, report.ledger_dir, Box::new(provider))
}

fn handed(ledger_dir: &std::path::Path) -> Vec<String> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter(|line| line["kind"] == "run_started")
        .filter_map(|line| line["addr"].as_str().map(str::to_owned))
        .collect()
}

fn unpaced() -> Pace {
    Arc::new(|_: &str| {})
}

type Gate = Arc<(Mutex<bool>, Condvar)>;

fn holding(key: &'static str, gate: &Gate) -> Pace {
    let gate = Arc::clone(gate);
    Arc::new(move |request: &str| {
        if !request.contains(key) {
            return;
        }
        let (opened, opening) = &*gate;
        drop(
            opening
                .wait_timeout_while(
                    opened.lock().unwrap(),
                    std::time::Duration::from_secs(10),
                    |opened| !*opened,
                )
                .unwrap(),
        );
    })
}

fn open(gate: &Gate) {
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
}

fn handed_back(ledger_dir: &std::path::Path) -> usize {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .filter(|line| line["kind"] == "signal_enqueued" && line["addr"] == PARENT)
        .count()
}

fn joined(worker: &RunWorker) -> usize {
    worker
        .collaborating
        .joins
        .get(&Address::parse(PARENT).unwrap())
        .map_or(0, |join| join.artifacts().count())
}

fn until_joined(worker: &mut RunWorker, count: usize) {
    while joined(worker) < count && worker.driving() {
        worker
            .serve_flight(crate::worker::relay::Patience::Unbounded)
            .unwrap();
    }
}

#[test]
fn vector_early_hands_every_node_once_in_dependency_order() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, ledger, _provider) = laid_out(dir.path(), &[], unpaced());
    worker.land_the_rest().unwrap();
    worker.end_hand_over(RunId::CITY, GraphAfter::Open).unwrap();
    let order = handed(&ledger);
    assert_eq!(order.len(), 4, "{order:?}");
    assert_eq!(order[0], "lab/n1");
    assert_eq!(
        order[1..3].iter().cloned().collect::<BTreeSet<_>>(),
        ["lab/n2".to_owned(), "lab/n3".to_owned()].into()
    );
    assert_eq!(order[3], "lab/n4");
    assert_eq!(joined(&worker), 4);
    assert!(
        !worker
            .collaborating
            .workshops
            .contains_key(&Address::parse(PARENT).unwrap())
    );
}

#[test]
fn vector_cancelled_hands_nothing_after_the_parent_ends() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, ledger, _provider) = laid_out(dir.path(), &[], unpaced());
    until_joined(&mut worker, 1);
    worker
        .end_hand_over(RunId::CITY, GraphAfter::Closed)
        .unwrap();
    worker.land_the_rest().unwrap();
    let order = handed(&ledger);
    assert_eq!(
        order.iter().cloned().collect::<BTreeSet<_>>(),
        [
            "lab/n1".to_owned(),
            "lab/n2".to_owned(),
            "lab/n3".to_owned()
        ]
        .into(),
        "{order:?}"
    );
    assert_eq!(order.len(), 3, "{order:?}");
    assert_eq!(joined(&worker), 3, "nodes in flight still hand back");
    assert!(
        !worker
            .collaborating
            .workshops
            .contains_key(&Address::parse(PARENT).unwrap())
    );
}

#[test]
fn vector_failed_hands_nothing_after_the_parent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let gate = Gate::default();
    let (mut worker, ledger, _provider) =
        laid_out(dir.path(), &[], holding("n3 is checked", &gate));
    until_joined(&mut worker, 2);
    let failed =
        Err(
            kernel::AxError::failure(kernel::AxCode::Provider, "drive the parent", PARENT)
                .with_recovery("none: the parent stands in for a failed drive"),
        );
    worker
        .end_hand_over(RunId::CITY, GraphAfter::of(&failed))
        .unwrap();
    open(&gate);
    worker.land_the_rest().unwrap();
    let order = handed(&ledger);
    assert_eq!(order.len(), 3, "{order:?}");
    assert_eq!(order[0], "lab/n1");
    assert_eq!(
        order[1..].iter().cloned().collect::<BTreeSet<_>>(),
        ["lab/n2".to_owned(), "lab/n3".to_owned()].into()
    );
    assert_eq!(joined(&worker), 3, "a node in flight still hands back");
    assert!(
        !worker
            .collaborating
            .workshops
            .contains_key(&Address::parse(PARENT).unwrap())
    );
}

#[test]
fn vector_stopped_leaves_the_graph_open_and_hands_no_more() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, ledger, _provider) = laid_out(dir.path(), &["n2"], unpaced());
    worker.end_hand_over(RunId::CITY, GraphAfter::Open).unwrap();
    worker.land_the_rest().unwrap();
    let order = handed(&ledger);
    assert_eq!(order.len(), 3, "{order:?}");
    assert_eq!(order[0], "lab/n1");
    assert_eq!(
        order[1..].iter().cloned().collect::<BTreeSet<_>>(),
        ["lab/n2".to_owned(), "lab/n3".to_owned()].into()
    );
    assert_eq!(handed_back(&ledger), 3, "the stopped node hands back too");
    assert_eq!(joined(&worker), 2, "only 1 and 3 join");
    assert!(
        worker
            .collaborating
            .workshops
            .contains_key(&Address::parse(PARENT).unwrap()),
        "the graph stays open"
    );
}
