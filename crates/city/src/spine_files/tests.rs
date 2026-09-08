// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::policy::BUILDING_FILE;
use kernel::{PlanTree, Progress, RoadmapShape, check_roadmap_shape};

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

fn roadmap_of(root: &Path) -> String {
    std::fs::read_to_string(root.join(ROADMAP_FILE)).unwrap()
}

#[test]
fn a_new_building_has_a_denominator_and_it_is_zero() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("lab");
    lay_out(&root, &addr("lab")).unwrap();

    let RoadmapShape::WellFormed { rows } = check_roadmap_shape(&roadmap_of(&root)) else {
        panic!("a laid-out roadmap parses");
    };
    assert!(
        rows.is_empty(),
        "the template's example rows are not this building's tasks"
    );
    let plan = PlanTree::build(rows).expect("a laid-out roadmap is a tree");
    let Progress::Planned(planned) = plan.progress() else {
        panic!("a building with a roadmap has a denominator");
    };
    assert_eq!(planned.ratio(), (0, 0));
    assert_eq!(planned.done_ppb, 0);
}

#[test]
fn the_documents_name_the_building_they_belong_to() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("lab");
    lay_out(&root, &addr("lab")).unwrap();
    for file in [ROADMAP_FILE, MEMO_FILE, HANDOFF_FILE] {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        assert!(text.contains("lab"), "{file} names its building");
        assert!(
            !text.contains(NAME_PLACEHOLDER),
            "{file} has no placeholder"
        );
    }
}

#[test]
fn a_building_that_has_been_working_keeps_its_plan() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("lab");
    lay_out(&root, &addr("lab")).unwrap();
    let worked = "| # | Item | Weight | Needs | Status | Evidence |\n\
                  |---|---|---|---|---|---|\n\
                  | 1 | ship it | 1 |  | Done | cas:b3-x |\n";
    std::fs::write(root.join(ROADMAP_FILE), worked).unwrap();

    lay_out(&root, &addr("lab")).unwrap();
    assert_eq!(roadmap_of(&root), worked);
}

#[test]
fn the_job_file_lands_in_the_room_and_says_what_the_run_was_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    let text = write_job(
        dir.path(),
        &room,
        &JobBrief {
            task: "measure the thing",
            goal: "a number with a unit, then stop",
            budget: "24 turns",
        },
    )
    .unwrap();

    let path = job_path(dir.path(), &room);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    assert!(path.ends_with(JOB_FILE));
    assert!(text.contains("measure the thing"));
    assert!(text.contains("a number with a unit, then stop"));
    assert!(text.contains("24 turns"));
}

#[test]
fn a_second_job_replaces_the_first_because_it_is_this_sessions_task() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    let brief = |task| JobBrief {
        task,
        goal: "stop when done",
        budget: "",
    };
    write_job(dir.path(), &room, &brief("first")).unwrap();
    let second = write_job(dir.path(), &room, &brief("second")).unwrap();

    let on_disk = std::fs::read_to_string(job_path(dir.path(), &room)).unwrap();
    assert_eq!(on_disk, second);
    assert!(!on_disk.contains("first"));
    assert!(
        !on_disk.contains("## Budget"),
        "a section with no fact behind it is not written"
    );
}

#[test]
fn a_stated_goal_is_what_makes_a_task_a_job() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    let brief = write_brief(
        dir.path(),
        &room,
        &JobBrief {
            task: "measure the thing",
            goal: "a number with a unit, then stop",
            budget: "24 turns",
        },
    )
    .unwrap();

    let RunBrief::Job { text } = &brief else {
        panic!("a dispatch that says when to stop is a job");
    };
    assert!(text.contains("a number with a unit, then stop"));
    assert_eq!(
        std::fs::read_to_string(job_path(dir.path(), &room)).unwrap(),
        *text,
        "the brief in the prefix and the file in the room are the same bytes"
    );
}

/// A session nobody assigned a goal to is a conversation, and no job
/// file is written for it: a form whose one irreplaceable field is
/// blank teaches an agent that stopping is undefined.
#[test]
fn a_session_with_no_goal_is_the_person_and_leaves_no_job_file() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    for empty in ["", "   ", "\n"] {
        let brief = write_brief(
            dir.path(),
            &room,
            &JobBrief {
                task: "what do you make of this",
                goal: empty,
                budget: "24 turns",
            },
        )
        .unwrap();
        assert_eq!(brief, RunBrief::Principal);
        assert!(
            !job_path(dir.path(), &room).exists(),
            "{empty:?} wrote a job file anyway"
        );
    }
    assert!(
        RunBrief::Principal.segment_text().contains("person"),
        "the brief says what to do instead, not that a file is missing"
    );
}

/// The prefix pays for every byte it carries, and a handoff nobody
/// filled in carries nothing.
#[test]
fn an_unfilled_handoff_is_not_worth_prefix_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("lab");
    let lab = addr("lab");
    assert_eq!(
        handoff(dir.path(), &lab).unwrap(),
        None,
        "no building, no handoff"
    );

    lay_out(&root, &lab).unwrap();
    assert_eq!(
        handoff(dir.path(), &lab).unwrap(),
        None,
        "the blank form is the absence of a handoff, not a handoff"
    );

    let written = "# Handoff — lab\n\n## 1 Must-read list\n\nRead the wire spec first.\n";
    std::fs::write(root.join(HANDOFF_FILE), written).unwrap();
    assert_eq!(handoff(dir.path(), &lab).unwrap().as_deref(), Some(written));

    // A third fact, and it is neither of the two above: the file is
    // there and cannot be read.
    let unreadable = dir.path().join("unreadable");
    std::fs::create_dir_all(unreadable.join("lab").join(HANDOFF_FILE)).unwrap();
    let err = handoff(&unreadable, &lab)
        .expect_err("a handoff that cannot be read is not an absent handoff");
    assert!(err.to_string().contains(HANDOFF_FILE), "{err}");
}

#[test]
fn the_norms_are_the_citys_and_the_buildings_in_that_order() {
    let dir = tempfile::tempdir().unwrap();
    let room = addr("lab/room1");
    std::fs::write(dir.path().join(CITY_FILE), "# City.md\n").unwrap();

    let before = norms(dir.path(), &room).unwrap();
    assert_eq!(before.len(), 1, "a building with no rules contributes none");

    crate::building::create(dir.path(), &addr("lab"), crate::BuildingTemplate::Minimal).unwrap();
    let after = norms(dir.path(), &room).unwrap();
    assert_eq!(after.len(), 2);
    assert!(after[0].ends_with(CITY_FILE));
    assert!(after[1].ends_with(BUILDING_FILE));
    assert!(after[1].starts_with(dir.path().join("lab")));
}

#[test]
fn the_reserved_subtree_has_no_norms_to_read() {
    let dir = tempfile::tempdir().unwrap();
    let err = norms(dir.path(), &addr(".sprawling/ledger")).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}
