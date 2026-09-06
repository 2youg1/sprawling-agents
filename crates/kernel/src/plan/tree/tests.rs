// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::completion::Progress;
use crate::error::{AxCode, AxError};
use crate::node_id::NodeId;
use crate::spine::{RoadmapShape, check_roadmap_shape};
fn tree(text: &str) -> PlanTree {
    let RoadmapShape::WellFormed { rows } = check_roadmap_shape(text) else {
        panic!("the fixture parses");
    };
    PlanTree::build(rows).expect("the fixture is a tree")
}
fn refused(text: &str) -> AxError {
    let RoadmapShape::WellFormed { rows } = check_roadmap_shape(text) else {
        panic!("the fixture parses");
    };
    PlanTree::build(rows).expect_err("the fixture is not a tree")
}

const HEAD: &str = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
";
#[test]
fn an_index_is_read_as_a_path_and_a_bad_one_is_named() {
    let id = NodeId::parse("2.3.1").unwrap();
    assert_eq!(id.depth(), 3);
    assert_eq!(id.parent().unwrap().as_str(), "2.3");
    assert_eq!(id.ordinal(), 1);
    assert!(NodeId::parse("2").unwrap().is_ancestor_of(&id));
    assert!(!NodeId::parse("2.3.1").unwrap().is_ancestor_of(&id));
    // `21` starts with `2` and is not below it: the check is on the
    // separator, not on the prefix.
    assert!(
        !NodeId::parse("2")
            .unwrap()
            .is_ancestor_of(&NodeId::parse("21").unwrap())
    );
    for bad in ["", "0", "1.0", "01", "1..2", "1.a", "-1"] {
        assert!(NodeId::parse(bad).is_err(), "`{bad}` is not an index");
    }
}

#[test]
fn the_reading_order_of_the_table_is_the_order_of_the_ids() {
    let mut ids: Vec<NodeId> = ["2", "1.10", "1.2", "1", "10"]
        .into_iter()
        .map(|raw| NodeId::parse(raw).unwrap())
        .collect();
    ids.sort();
    let drawn: Vec<&str> = ids.iter().map(NodeId::as_str).collect();
    assert_eq!(drawn, ["1", "1.10", "1.2", "10", "2"]);
}

#[test]
fn progress_counts_leaves_and_reports_both_figures() {
    let plan = tree(&format!(
        "{HEAD}\
| 1 | build | 1 |  | In progress | |
| 1.1 | design | 1 |  | Done | cas:b3-{h} |
| 1.2 | code | 1 |  | Blocked | |
| 2 | ship | 1 |  | Not started | |
",
        h = "ab".repeat(32)
    ));
    let Progress::Planned(planned) = plan.progress() else {
        panic!("a tree reports planned progress");
    };
    assert_eq!(
        (planned.done, planned.blocked, planned.total),
        (1, 1, 3),
        "the branch itself is not a leaf and is not counted"
    );
    assert_eq!(planned.done_ppb, 250_000_000);
    assert_eq!(planned.blocked_ppb, 250_000_000);
}

#[test]
fn a_done_row_without_evidence_reaches_neither_figure() {
    let plan = tree(&format!(
        "{HEAD}\
| 1 | claimed without evidence | 1 |  | Done | |
| 2 | ship | 1 |  | Not started | |
"
    ));
    let Progress::Planned(planned) = plan.progress() else {
        panic!("planned")
    };
    assert_eq!(planned.done, 0);
    assert_eq!(planned.done_ppb, 0);
}

#[test]
fn the_ready_set_is_what_nobody_holds_and_nothing_blocks() {
    let plan = tree(&format!(
        "{HEAD}\
| 1 | design | 1 |  | Done | cas:b3-{h} |
| 2 | code | 1 | 1 | Not started | |
| 3 | ship | 1 | 2 | Not started | |
| 4 | notes | 1 |  | In progress | |
",
        h = "ab".repeat(32)
    ));
    let ready: Vec<String> = plan.ready().iter().map(NodeId::to_string).collect();
    assert_eq!(ready, ["2"], "3 waits for 2, and 4 is taken");
}

#[test]
fn a_child_waits_for_what_its_branch_waits_for() {
    let plan = tree(&format!(
        "{HEAD}\
| 1 | groundwork | 1 |  | Not started | |
| 2 | build | 1 | 1 | Not started | |
| 2.1 | frame | 1 |  | Not started | |
"
    ));
    let ready: Vec<String> = plan.ready().iter().map(NodeId::to_string).collect();
    assert_eq!(
        ready,
        ["1"],
        "2.1 inherits 2's dependency on 1, which is not done"
    );
    let refusal = plan.claim(&NodeId::parse("2.1").unwrap()).unwrap_err();
    assert!(refusal.subject().contains("waits for 1"));
}

#[test]
fn a_circle_is_refused_where_it_is_written_and_the_walk_is_named() {
    let refusal = refused(&format!(
        "{HEAD}\
| 1 | a | 1 | 3 | Not started | |
| 2 | b | 1 | 1 | Not started | |
| 3 | c | 1 | 2 | Not started | |
"
    ));
    assert_eq!(refusal.code(), &AxCode::InvalidArgs);
    assert!(refusal.subject().contains('→'), "{}", refusal.subject());
    assert!(refusal.recovery().contains("circle"));
}

#[test]
fn a_node_that_needs_itself_is_refused_by_name() {
    let refusal = refused(&format!(
        "{HEAD}\
| 1 | a | 1 | 1 | Not started | |
"
    ));
    assert!(refusal.subject().contains("needs itself"));
}

#[test]
fn a_dependency_on_a_row_nobody_wrote_is_refused() {
    let refusal = refused(&format!(
        "{HEAD}\
| 1 | a | 1 | 9 | Not started | |
"
    ));
    assert!(refusal.subject().contains("does not carry"));
}

#[test]
fn a_branch_with_no_parent_row_is_refused() {
    let refusal = refused(&format!(
        "{HEAD}\
| 2.1 | orphan | 1 |  | Not started | |
"
    ));
    assert!(refusal.subject().contains("hangs under 2"));
}

#[test]
fn a_branch_cannot_say_done_over_a_child_that_is_not() {
    let refusal = refused(&format!(
        "{HEAD}\
| 1 | build | 1 |  | Done | cas:b3-{h} |
| 1.1 | frame | 1 |  | Not started | |
",
        h = "ab".repeat(32)
    ));
    assert!(refusal.subject().contains("says done while 1.1"));
}

#[test]
fn a_repeated_index_is_two_plans_and_is_refused() {
    let refusal = refused(&format!(
        "{HEAD}\
| 1 | a | 1 |  | Not started | |
| 1 | b | 1 |  | Not started | |
"
    ));
    assert!(refusal.subject().contains("twice"));
}

/// Claiming is the only way to get a `Held`, and a branch, a taken
/// node and a waiting node are each refused with a different reason.
/// Claiming is the only way to get a `Held`, and a branch, a taken
/// node and a waiting node are each refused with a different reason.
#[test]
fn claiming_names_which_of_the_three_reasons_applies() {
    let plan = tree(&format!(
        "{HEAD}\
| 1 | build | 1 |  | In progress | |
| 1.1 | frame | 1 |  | Not started | |
| 2 | ship | 1 | 1.1 | Not started | |
"
    ));
    assert!(
        plan.claim(&NodeId::parse("1").unwrap())
            .unwrap_err()
            .subject()
            .contains("branch")
    );
    assert!(
        plan.claim(&NodeId::parse("2").unwrap())
            .unwrap_err()
            .subject()
            .contains("waits for 1.1")
    );
    assert_eq!(
        plan.claim(&NodeId::parse("1").unwrap()).unwrap_err().code(),
        &AxCode::InvalidArgs,
        "a branch is a mistake about the plan's shape"
    );
    assert!(
        plan.claim(&NodeId::parse("9").unwrap())
            .unwrap_err()
            .subject()
            .contains("no row")
    );
    let held = plan.claim(&NodeId::parse("1.1").unwrap()).unwrap();
    assert_eq!(held.id().as_str(), "1.1");
}

#[test]
fn a_refusal_points_at_a_node_the_caller_may_take() {
    let plan = tree(&format!(
        "{HEAD}\
| 1 | taken | 1 |  | In progress | |
| 2 | free | 1 |  | Not started | |
"
    ));
    let refusal = plan.claim(&NodeId::parse("1").unwrap()).unwrap_err();
    assert_eq!(
        refusal.code(),
        &AxCode::GoalConflict,
        "two runs wanting one node is a goal conflict"
    );
    assert!(
        refusal.recovery().contains("claim 2"),
        "{}",
        refusal.recovery()
    );
}

/// The plan gate: a held node leaves green with evidence or stopped
/// with a cause, and handing back is the one stop that is not red.
#[test]
fn an_empty_plan_is_a_tree_with_nothing_ready() {
    let plan = PlanTree::build(Vec::new()).unwrap();
    assert!(plan.is_empty());
    assert!(plan.ready().is_empty());
    let Progress::Planned(planned) = plan.progress() else {
        panic!("planned")
    };
    assert_eq!(planned.total, 0);
    assert_eq!(planned.done_ppb, 0);
}
