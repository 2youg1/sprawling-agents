// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn node(id: &str, deps: &[&str]) -> NodeContract {
    NodeContract::new(
        NodeId::parse(id).unwrap(),
        format!("produce {id}"),
        deps.iter()
            .map(|d| NodeId::parse(d).unwrap())
            .collect::<BTreeSet<NodeId>>(),
        Vec::new(),
        Address::parse(&format!("lab/{id}")).unwrap(),
        "lab/room1".to_owned(),
        "the test suite passes".to_owned(),
        "stop when the check passes".to_owned(),
    )
    .unwrap()
}

#[test]
fn the_same_graph_schedules_the_same_way_every_time() {
    let first = Workshop::new(vec![
        node("c", &["a", "b"]),
        node("a", &[]),
        node("b", &["a"]),
    ])
    .unwrap();
    // The same nodes, handed over in a different order.
    let second = Workshop::new(vec![
        node("b", &["a"]),
        node("c", &["a", "b"]),
        node("a", &[]),
    ])
    .unwrap();

    let order: Vec<String> = first
        .schedule()
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    assert_eq!(order, ["a", "b", "c"]);
    assert_eq!(first.schedule(), second.schedule());
}

#[test]
fn independent_nodes_are_ready_together_and_that_is_the_fan_out() {
    let workshop =
        Workshop::new(vec![node("a", &[]), node("b", &[]), node("c", &["a", "b"])]).unwrap();
    let ready: Vec<String> = workshop
        .ready(&BTreeSet::new())
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    assert_eq!(ready, ["a", "b"]);

    let done: BTreeSet<NodeId> = ["a", "b"]
        .iter()
        .map(|d| NodeId::parse(d).unwrap())
        .collect();
    let ready: Vec<String> = workshop
        .ready(&done)
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    assert_eq!(ready, ["c"]);
}

#[test]
fn a_cycle_is_refused_at_construction_and_names_the_nodes_in_it() {
    let err = Workshop::new(vec![node("a", &["b"]), node("b", &["a"])]).unwrap_err();
    assert_eq!(err.code(), &AxCode::GoalConflict);
    assert!(err.subject().contains('a') && err.subject().contains('b'));
}

#[test]
fn a_dependency_on_a_node_nobody_wrote_is_refused() {
    let err = Workshop::new(vec![node("a", &["ghost"])]).unwrap_err();
    assert!(err.subject().contains("ghost"));
    assert!(err.recovery().contains("drop the dependency"));
}

#[test]
fn two_contracts_cannot_claim_one_node() {
    let err = Workshop::new(vec![node("a", &[]), node("a", &[])]).unwrap_err();
    assert!(err.recovery().contains("rename one"));
}

#[test]
fn a_contract_that_leaves_the_stop_condition_to_the_reader_is_refused() {
    let err = NodeContract::new(
        NodeId::parse("a").unwrap(),
        "produce a".to_owned(),
        BTreeSet::new(),
        Vec::new(),
        Address::parse("lab/a").unwrap(),
        "lab/room1".to_owned(),
        "the suite passes".to_owned(),
        "   ".to_owned(),
    )
    .unwrap_err();
    assert!(err.subject().contains("stop"));
}

#[test]
fn the_contract_is_the_job_file_so_the_agent_reads_one_authority() {
    let contract = node("a", &[]);
    let text = contract.job_text();
    assert!(text.contains("## Goal"));
    assert!(text.contains("## Done check"));
    assert!(text.contains("the test suite passes"));
    assert!(text.contains("## Stop"));
    assert!(text.contains("lab/a"), "the node writes where it may write");
}
