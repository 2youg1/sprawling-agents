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

//! The derived check of `crates/collab/spec/Workshop.lean`: its four
//! trace vectors replayed through the production hand-over, with the
//! parent standing in as a desk pair and its ending given as
//! `GraphAfter`, and the children as real runs that land and hand back.
//!
//! Every vector is replayed through the same doors production uses:
//! `RunWorker::hand_over_at_call`, the child runs' landings and
//! handbacks, and `end_hand_over`. `laid_out` builds the drive once for
//! all four.

use std::collections::BTreeSet;
use std::sync::{Arc, Condvar, Mutex};

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::waking::handing::{GraphAfter, Handing};

const PARENT: &str = "lab/room1";

/// The diamond of the model: 1 first, 2 and 3 after 1, 4 after both.
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

/// A city with the building, a worker whose provider ends every node
/// Done except those named in `stopping`, whose empty reply ends them
/// Limit and so hands back `Handback::Stopped`, every model call
/// passed through `pace` first, and the parent's graph laid out on
/// its desks and handed to the city the way a call is:
/// `hand_over_at_call`.
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
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let parent = Address::parse(PARENT).unwrap();
    let mut delegates =
        collab::DelegateDesk::new(kernel::Depth::Root, Address::parse("lab").unwrap());
    let mut workshop =
        collab::WorkshopDesk::new(PARENT.to_owned(), collab::FanIn::new(), BTreeSet::new());
    workshop.lay_out(diamond(), &mut delegates).unwrap();
    let at = Assignment {
        addr: parent,
        session: None,
        effort: None,
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

/// The node rooms in the order their runs started: the model's
/// `handed`.
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

/// A pace that lets every model call through.
fn unpaced() -> Pace {
    Arc::new(|_: &str| {})
}

/// Opened once by the test; until then a held model call waits.
type Gate = Arc<(Mutex<bool>, Condvar)>;

/// A pace that holds every model call naming `key` until `gate`
/// opens, so the node's landing follows a step the test takes
/// rather than whichever lane is faster.
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

/// How many handbacks reached the parent's room: the model's `backs`,
/// which counts a stopped node as well as a finished one.
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

/// Serves the crossing until `count` nodes have joined, or until no
/// run is left driving, so an implementation that hands nothing down
/// fails the test's assertions instead of hanging it.
fn until_joined(worker: &mut RunWorker, count: usize) {
    while joined(worker) < count && worker.driving() {
        worker
            .serve_flight(crate::worker::relay::Patience::Unbounded)
            .unwrap();
    }
}

/// `vectorEarly`: every node lands before the parent ends Done, and
/// the graph hands 1, then 2 and 3, then 4, and closes when joined.
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

/// `vectorCancelled`: node 1 lands, the parent is cancelled, 2 and 3
/// still run to the end and hand back, and 4 is never handed down.
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

/// `vectorFailed`: 1 and 2 land, the parent fails, and 3, still in
/// flight, hands back after it, so 4 is never handed down. Node 3's
/// model call is held until the parent has ended, which makes the
/// order the vector's rather than the lanes'.
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

/// `vectorStopped`: the parent ends Limit first, which leaves the
/// graph open; 1 finishes, 2 stops, 3 finishes. All three hand back,
/// but the stopped node does not join, so 4 never becomes ready and
/// the graph stays registered for a join that cannot come. This is
/// the behaviour the model states while the open question of
/// `crates/collab/Spec.lean` §3, what a stopped node does to its
/// graph, stands; settling it changes the model first and this test
/// with it. The model's last act, a second landing of 2, has no
/// production door: a run lands once.
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
