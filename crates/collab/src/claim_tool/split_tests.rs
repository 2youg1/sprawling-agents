// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::Tool;
use kernel::spine::set_roadmap_status;

use super::tests::{PLAN, call, desk, node};
use super::*;

#[test]
fn splitting_grows_the_plan_and_the_run_stops_holding_the_branch() {
    let shared = desk();
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let outcome = tool
        .invoke(&call(serde_json::json!({
            "action": "split",
            "node": "1",
            "parts": [{"item": "run the cable", "weight": 3}, "test the element"]
        })))
        .unwrap();
    assert_eq!(
        outcome
            .result
            .as_map()
            .get("children")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    let unfinished = outcome.result.as_map().get("unfinished").cloned();
    assert_eq!(
        unfinished,
        Some(Value::from(2)),
        "the split reports what is left"
    );
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(text.contains("| 1.1 | run the cable | 3 |  | Not started |  |"));
    assert!(
        text.contains("| 1.2 | test the element | 1 |  | Not started |  |"),
        "a bare string is a child of weight one"
    );
    assert!(
        shared.lock().unwrap().holding().is_none(),
        "the work it took is now several pieces; it takes one of them next"
    );
    let effects = shared.lock().unwrap().take_effects();
    assert_eq!(effects[1].kind(), Some(kernel::EventKind::RoadmapSplit));
}

/// The eighth finding of `tools/adversary/Spec.lean`: a split of a row
/// nobody held was answered as done and then dropped at landing. The
/// desk now refuses it at the call (collab D6), queues nothing and leaves
/// the plan as the claim left it, and the refusal names what the run
/// holds and the claim that would make the split possible.
#[test]
fn a_split_of_a_row_this_run_does_not_hold_is_refused_at_the_call() {
    let split = |id: &str| {
        call(serde_json::json!({
            "action": "split", "node": id, "parts": ["run the cable", "test the element"]
        }))
    };
    // What a desk queued and wrote, and what it still holds.
    let left = |shared: &Arc<Mutex<ClaimDesk>>| {
        let mut desk = shared.lock().unwrap();
        let kinds: Vec<_> = desk
            .take_effects()
            .iter()
            .filter_map(ClaimEffect::kind)
            .collect();
        (
            kinds,
            desk.roadmap().map(str::to_owned),
            desk.holding().cloned(),
        )
    };
    let idle = desk();
    let unheld = ClaimTool::new(Arc::clone(&idle))
        .unwrap()
        .invoke(&split("1"))
        .err();
    let busy = desk();
    let tool = ClaimTool::new(Arc::clone(&busy)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .unwrap();
    let elsewhere = tool.invoke(&split("3")).err();
    let claimed = set_roadmap_status(PLAN, &node("1"), RoadmapStatus::InProgress, None).unwrap();

    assert_eq!(
        (unheld, left(&idle), elsewhere, left(&busy)),
        (
            Some(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "split a plan node",
                    "this run holds nothing, so it cannot split 1",
                )
                .with_recovery("claim 1 first; a row is divided by the run that holds it")
            ),
            (Vec::new(), None, None),
            Some(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "split a plan node",
                    "this run holds 1, not 3",
                )
                .with_recovery("split 1, or put it down and claim 3 first")
            ),
            (
                vec![kernel::EventKind::RoadmapClaimed],
                Some(claimed),
                Some(node("1"))
            )
        ),
        "a split the desk cannot land is refused before anything is queued"
    );
}

/// A dependency a split writes is a real edge: the child stays out of
/// the ready set until the row it names is done, the claim says which
/// row it waits for, and finishing that row lets the child in. The
/// grown table is judged by `PlanTree::build` before a byte is written,
/// so a name the plan does not carry is refused at the call.
#[test]
fn a_split_writes_its_dependencies_and_the_ready_set_reads_them() {
    let plan = format!(
        "# Roadmap\n\n| # | Item | Weight | Needs | Status | Evidence |\n\
         |---|------|--------|-------|--------|----------|\n\
         | 1 | foundations | 1 |  | Done | cas:b3-{h} |\n\
         | 2 | build | 1 |  | Not started |  |\n\
         | 3 | ship | 1 | 1 | Not started |  |\n",
        h = "ab".repeat(32)
    );
    let shared = Arc::new(Mutex::new(ClaimDesk::new(
        "potter@lab.1".to_owned(),
        Address::parse("lab/room1").unwrap(),
        plan,
        Booking::new(|_| Ok(())),
    )));
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let ready = |tool: &ClaimTool| {
        let listed = tool
            .invoke(&call(serde_json::json!({ "action": "list" })))
            .expect("the plan lists")
            .result;
        listed
            .as_map()
            .get("ready")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.get("node").and_then(Value::as_str))
                    .map(str::to_owned)
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default()
    };
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "2" })))
        .expect("2 is ready: it waits for nothing, and 1 is done");
    tool.invoke(&call(serde_json::json!({
        "action": "split",
        "node": "2",
        "parts": [{"item": "frame", "needs": ["3"]}, {"item": "roof", "needs": ""}]
    })))
    .expect("the split lands on the desk");
    let text = shared.lock().unwrap().roadmap().unwrap().to_owned();
    assert!(
        text.contains("| 2.1 | frame | 1 | 3 | Not started |  |"),
        "the needs column carries the edge: {text}"
    );
    assert!(
        text.contains("| 2.2 | roof | 1 |  | Not started |  |"),
        "an empty `needs` writes an empty cell: {text}"
    );
    assert_eq!(ready(&tool), ["2.2", "3"], "2.1 waits for 3");
    let refused = tool
        .invoke(&call(
            serde_json::json!({ "action": "claim", "node": "2.1" }),
        ))
        .expect_err("2.1 waits for 3, which nobody has finished");
    assert!(
        refused.subject().contains("waits for 3"),
        "the refusal names the row being waited for: {}",
        refused.subject()
    );
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "3" })))
        .expect("3 is ready");
    tool.invoke(&call(serde_json::json!({
        "action": "finish",
        "node": "3",
        "evidence": format!("cas:b3-{}", "cd".repeat(32)),
    })))
    .expect("3 lands with a locator");
    assert_eq!(
        ready(&tool),
        ["2.1", "2.2"],
        "finishing the row 2.1 waits for lets it in"
    );
}

/// A dependency the grown table cannot hang is refused at the call, by
/// the same judgement that reads the plan, and the desk keeps the copy
/// it had.
#[test]
fn a_split_that_needs_a_row_the_plan_lacks_is_refused_before_it_is_written() {
    let shared = desk();
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    tool.invoke(&call(serde_json::json!({ "action": "claim", "node": "1" })))
        .expect("1 is ready");
    let before = shared.lock().unwrap().roadmap().map(str::to_owned);
    let refused = tool
        .invoke(&call(serde_json::json!({
            "action": "split",
            "node": "1",
            "parts": [{"item": "frame", "needs": ["9"]}]
        })))
        .expect_err("row 9 is not in the plan");
    assert_eq!(
        refused.subject(),
        "1.1 needs 9, which the table does not carry",
        "the refusal names the row and the dependency"
    );
    assert_eq!(
        shared.lock().unwrap().roadmap().map(str::to_owned),
        before,
        "a refused split leaves the desk's copy as the claim left it"
    );
    assert!(
        shared
            .lock()
            .unwrap()
            .take_effects()
            .iter()
            .all(|effect| effect.kind() != Some(kernel::EventKind::RoadmapSplit)),
        "and queues no split"
    );
}

/// A plan with a header and no rows: what a building starts with.
const EMPTY_PLAN: &str = "\
# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
";

fn empty_desk(room: &str) -> Arc<Mutex<ClaimDesk>> {
    Arc::new(Mutex::new(ClaimDesk::new(
        "mayor@hall.1".to_owned(),
        Address::parse(room).unwrap(),
        EMPTY_PLAN.to_owned(),
        Booking::new(|_| Ok(())),
    )))
}

/// An empty plan had no first line, because every action
/// named a node and the root's share was given to nobody. The root is
/// the building's Mayor's (kernel `Share.lean` D25), so its `add` with
/// no node writes a top-level row, and the leaves still gather to the
/// whole plan.
#[test]
fn the_mayor_writes_the_first_line_of_an_empty_plan() {
    let shared = empty_desk(kernel::consts_policy::HALL_MAYOR);
    let tool = ClaimTool::new(Arc::clone(&shared)).unwrap();
    let added = tool
        .invoke(&call(serde_json::json!({
            "action": "add", "parts": [{"item": "survey the river", "weight": 2}]
        })))
        .ok()
        .map(|outcome| outcome.result.as_map().get("nodes").cloned());
    let text = shared.lock().unwrap().roadmap().map(str::to_owned);
    let whole = text.as_deref().and_then(|text| {
        let RoadmapShape::WellFormed { rows } = check_roadmap_shape(text) else {
            return None;
        };
        let tree = PlanTree::build(rows).ok()?;
        let leaves: Vec<kernel::Share> = tree
            .nodes()
            .filter(|node| node.is_leaf())
            .map(|node| node.share)
            .collect();
        Some(kernel::share::gather(&leaves))
    });
    assert_eq!(
        (added, text, whole),
        (
            Some(Some(serde_json::json!(["1"]))),
            Some(format!(
                "{EMPTY_PLAN}| 1 | survey the river | 2 |  | Not started |  |\n"
            )),
            Some(kernel::Share::WHOLE)
        ),
        "the first line is a top-level row, and it holds the whole plan"
    );
}

/// Only the root's holder adds under the root: a run anywhere else adds
/// under the branch it holds, which is what `split` already does.
#[test]
fn a_run_that_is_not_the_mayor_cannot_add_under_the_root() {
    let shared = empty_desk("lab/room1");
    let refused = ClaimTool::new(Arc::clone(&shared))
        .unwrap()
        .invoke(&call(
            serde_json::json!({ "action": "add", "parts": ["survey the river"] }),
        ))
        .err();
    assert_eq!(
        (refused, shared.lock().unwrap().roadmap().map(str::to_owned)),
        (
            Some(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "add a plan row",
                    "the plan's root belongs to the building's Mayor, and this run is lab/room1",
                )
                .with_recovery(
                    "claim a row and split it to add work under it, or signal hall/mayor to add \
                     a top-level row"
                )
            ),
            None
        )
    );
}
