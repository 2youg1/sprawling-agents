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

//! What `delegate` does the moment it is called (collab D7): the child
//! starts while its parent still works.

use super::super::*;
use super::tests::move_in;
use crate::worker::fixture::*;

/// The provider holds the parent's second call until a third call
/// arrives, so the child's first call has to come from a run started
/// while the parent drives: what is measured is where the child is
/// started, not which lane wins a race.
fn holding_the_second_call_for_a_third() -> Pace {
    let calls = std::sync::Arc::new((std::sync::Mutex::new(0u32), std::sync::Condvar::new()));
    std::sync::Arc::new(move |request: &str| {
        if !request.starts_with("POST ") {
            return;
        }
        let (count, arrived) = &*calls;
        let mut count = count.lock().unwrap();
        *count += 1;
        arrived.notify_all();
        if *count == 2 {
            drop(
                arrived
                    .wait_timeout_while(count, std::time::Duration::from_secs(10), |seen| *seen < 3)
                    .unwrap(),
            );
        }
    })
}

fn position_of(lines: &[serde_json::Value], kind: &str, room: &str) -> Option<usize> {
    lines
        .iter()
        .position(|line| line["kind"] == kind && line.to_string().contains(&format!("\"{room}\"")))
}

#[test]
fn a_delegated_child_starts_before_its_parent_freezes() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    move_in(dir.path(), "lab/lead");
    let (base_url, _provider) = fake_openai_paced(
        &["m-local"],
        Vec::new(),
        vec![
            tool_completion(
                "handing it down",
                "tu_1",
                "delegate",
                serde_json::json!({
                    "room": "lab/helper",
                    "task": "measure the thing",
                    "goal": "a number, then stop",
                }),
            ),
            completion("handed down, carrying on", None),
            completion("measured", None),
            completion("read what came back", None),
        ],
        holding_the_second_call_for_a_third(),
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/lead").unwrap(),
            task: "get it measured".to_owned(),
            goal: "the number is written down, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    worker.land_the_rest().unwrap();
    let lines: Vec<serde_json::Value> = runtime::replay::verify_ledger_dir(&report.ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
        .collect();
    let (child, parent) = (
        position_of(&lines, "run_started", "lab/helper"),
        position_of(&lines, "run_frozen", "lab/lead"),
    );
    assert!(
        matches!((child, parent), (Some(child), Some(parent)) if child < parent),
        "the child starts at the call: {child:?} {parent:?}"
    );
}

/// The derived check of `crates/collab/spec/Workshop.lean`: its trace
/// vectors replayed through the production hand-over, with the parent
/// standing in as a desk pair and its ending given as `GraphAfter`, and
/// the children as real runs that land and hand back.
mod vectors {
    use std::collections::BTreeSet;
    use std::sync::{Arc, Mutex};

    use super::super::super::*;
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
    /// Done, and the parent's graph laid out on its desks and handed to
    /// the city the way a call is: `hand_over_at_call`.
    fn laid_out(dir: &std::path::Path) -> (RunWorker, std::path::PathBuf, Box<dyn std::any::Any>) {
        let report = crate::worker::fixture::init_city(dir).unwrap();
        let routes = ["n1", "n2", "n3", "n4"]
            .map(|id| (format!("{id} is checked"), vec![completion("done", None)]));
        let (base_url, provider) = fake_openai_routed(
            &["m-local"],
            routes
                .iter()
                .map(|(key, replies)| (key.as_str(), replies.clone()))
                .collect(),
            vec![completion("done", None)],
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

    fn joined(worker: &RunWorker) -> usize {
        worker
            .collaborating
            .joins
            .get(&Address::parse(PARENT).unwrap())
            .map_or(0, |join| join.artifacts().count())
    }

    /// Serves the crossing until `count` nodes have joined.
    fn until_joined(worker: &mut RunWorker, count: usize) {
        while joined(worker) < count {
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
        let (mut worker, ledger, _provider) = laid_out(dir.path());
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
        let (mut worker, ledger, _provider) = laid_out(dir.path());
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
}
