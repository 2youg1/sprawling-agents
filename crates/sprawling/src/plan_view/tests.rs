// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use kernel::{EventDraft, GENESIS_PREV, Payload, RunId, Seq, TimeMs};

const PLAN: &str = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | groundwork | 1 |  | Blocked |  |
| 2 | build | 1 | 1 | Not started |  |
| 3 | ship | 1 | 2 | Not started |  |
";

fn city(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("lab");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Roadmap.md"), text).unwrap();
    dir
}

fn addr() -> Address {
    Address::parse("lab").unwrap()
}

fn record(kind: EventKind, data: serde_json::Value) -> EventRecord {
    let draft = EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(0),
        who: "mason@lab.1".into(),
        addr: Some(Address::parse("lab/room1").unwrap()),
        kind,
        data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::FIRST, GENESIS_PREV)
}

#[test]
fn a_plan_is_parsed_once_and_a_record_that_moves_it_makes_it_read_again() {
    let dir = city(PLAN);
    let mut view = PlanView::default();
    let first = view.of(dir.path(), &addr());
    assert_eq!(first.rows.len(), 3);

    // Nothing has said the plan moved, so a rewritten file is not
    // seen: that is the whole saving, and it is only correct
    // because every writer leaves a record.
    std::fs::write(dir.path().join("lab").join("Roadmap.md"), "gone").unwrap();
    assert_eq!(view.of(dir.path(), &addr()).rows.len(), 3);

    view.apply(&record(
        EventKind::RoadmapSplit,
        serde_json::json!({"node": "1", "by": "mason@lab.1"}),
    ));
    assert!(
        view.of(dir.path(), &addr()).rows.is_empty(),
        "the record sent it back to the file"
    );
}

/// A tool wave is a reason to read again even though it says nothing
/// about the plan: an agent that edited the table with the edit tool
/// leaves no `roadmap_*` record behind it.
#[test]
fn a_checkpoint_sends_the_plan_back_to_the_file() {
    let dir = city(PLAN);
    let mut view = PlanView::default();
    assert_eq!(view.of(dir.path(), &addr()).rows.len(), 3);
    std::fs::write(
        dir.path().join("lab").join("Roadmap.md"),
        PLAN.replace("| 3 | ship | 1 | 2 | Not started |  |\n", ""),
    )
    .unwrap();
    view.apply(&record(
        EventKind::CheckpointCommitted,
        serde_json::json!({}),
    ));
    assert_eq!(view.of(dir.path(), &addr()).rows.len(), 2);
}

#[test]
fn the_reason_a_node_is_red_comes_from_the_record_and_the_status_from_the_table() {
    let dir = city(PLAN);
    let mut view = PlanView::default();
    let bare = view.of(dir.path(), &addr());
    assert_eq!(bare.blocked.len(), 1, "one cause, not three symptoms");
    assert!(
        bare.blocked[0].line.contains("the plan says `Blocked`"),
        "with no record behind it the status word is the reason: {}",
        bare.blocked[0].line
    );
    assert_eq!(bare.blocked[0].waiting, 2, "2 and 3 stand behind it");

    view.apply(&record(
        EventKind::RoadmapBlocked,
        serde_json::json!({
            "node": "1", "by": "mason@lab.1",
            "why": {"blocked": {"note": "the quarry is shut"}}
        }),
    ));
    let told = view.of(dir.path(), &addr());
    assert_eq!(
        told.blocked[0].line,
        "branch 1 is stuck at 1: the quarry is shut"
    );
}

/// V3.20's closing condition: the projection holds nothing of its
/// own, so throwing it away and folding the same records again
/// produces the same reading.
#[test]
fn deleting_the_projection_and_folding_again_gives_the_same_reading() {
    let dir = city(PLAN);
    let history = [
        record(
            EventKind::RoadmapClaimed,
            serde_json::json!({"node": "2", "by": "mason@lab.1"}),
        ),
        record(
            EventKind::RoadmapBlocked,
            serde_json::json!({
                "node": "1", "by": "mason@lab.1",
                "why": {"blocked": {"note": "the quarry is shut"}}
            }),
        ),
    ];
    let mut first = PlanView::default();
    let mut second = PlanView::default();
    for held in &history {
        first.apply(held);
        second.apply(held);
    }
    let left = first.of(dir.path(), &addr());
    let right = second.of(dir.path(), &addr());
    assert_eq!(
        serde_json::to_string(&left.blocked).unwrap(),
        serde_json::to_string(&right.blocked).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&left.rows).unwrap(),
        serde_json::to_string(&right.rows).unwrap()
    );
}

#[test]
fn a_plan_that_does_not_parse_reports_the_problem_and_no_denominator() {
    let dir = city("no table here");
    let mut view = PlanView::default();
    let reading = view.of(dir.path(), &addr());
    assert!(matches!(reading.progress, Progress::Unplanned(_)));
    assert!(!reading.problems.is_empty());
    assert!(reading.rows.is_empty());
}

#[test]
fn a_dependency_circle_is_a_problem_rather_than_a_silent_empty_plan() {
    let dir = city(
        "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | a | 1 | 2 | Not started |  |
| 2 | b | 1 | 1 | Not started |  |
",
    );
    let mut view = PlanView::default();
    let reading = view.of(dir.path(), &addr());
    assert!(
        reading.problems.iter().any(|why| why.contains("circle")),
        "{:?}",
        reading.problems
    );
}
