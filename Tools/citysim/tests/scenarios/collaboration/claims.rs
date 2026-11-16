// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code,
    reason = "test code; readings are read through their Debug form in the report"
)]

use std::sync::Arc;

use kernel::layout::CityLayout;
use kernel::{Address, EventKind, EventRecord};

use super::board::{
    Board, CHECKER, GLAZER, MAKER, NOTES, NOTES_BEFORE, POTTER, Planners, REDO, plan_lab,
    plan_trace, room_of,
};
use super::ledger::{
    Lines, Mark, city, dispatch, frozen, quiet, raise, serve, serve_in_turn, started,
};

#[test]
fn a_node_two_residents_read_as_ready_in_one_wave_is_held_by_the_first_to_claim() {
    let dir = tempfile::tempdir().unwrap();
    let board = Arc::new(Board::default());
    let (mut worker, ledger) = city(dir.path(), Planners(Arc::clone(&board)));
    raise(&mut worker, "lab");
    plan_lab(dir.path());
    let lines = serve(
        worker,
        &ledger,
        vec![dispatch("lab/kiln", POTTER), dispatch("lab/glaze", GLAZER)],
        |lines| frozen(lines) == 2 && quiet(lines),
    );
    let plan = std::fs::read_to_string(
        CityLayout::new(dir.path())
            .scope(&Address::parse("lab").unwrap())
            .join(kernel::ROADMAP_FILE),
    )
    .unwrap();
    println!("{plan}");
    assert_eq!(
        plan_trace(&lines, |line| !matches!(
            line.0.kind(),
            EventKind::RunStarted | EventKind::RunFrozen
        )),
        [
            "RoadmapClaimed lab/kiln",
            "ToolResult lab/kiln plan answered",
            "ToolResult lab/glaze plan failed",
            "RoadmapBlocked lab/kiln",
        ],
        "one holder, the second claim refused at the call"
    );
}

#[test]
fn a_room_delegated_again_after_its_child_failed_starts_under_the_same_parent() {
    let dir = tempfile::tempdir().unwrap();
    let board = Arc::new(Board::default());
    let (mut worker, ledger) = city(dir.path(), Planners(Arc::clone(&board)));
    raise(&mut worker, "lab");
    plan_lab(dir.path());
    *board.ledger.lock().unwrap() = Some(ledger.clone());
    let lines = serve(worker, &ledger, vec![dispatch("lab/lead", REDO)], |lines| {
        frozen(lines) == 3 && quiet(lines)
    });
    let lead = started(&lines, "lab/lead").remove(0).run;
    let kiln = |line: &(EventRecord, String)| {
        line.0
            .addr()
            .map(Address::as_str)
            .or_else(|| room_of(&lines, line.0.run()))
            == Some("lab/kiln")
    };
    let parents: Vec<bool> = started(&lines, "lab/kiln")
        .iter()
        .map(|start| start.parent == Some(lead))
        .collect();
    assert_eq!(
        (parents, plan_trace(&lines, kiln)),
        (
            vec![true, true],
            [
                "RunStarted lab/kiln",
                "RoadmapClaimed lab/kiln",
                "ToolResult lab/kiln plan answered",
                "ToolResult lab/kiln run failed",
                "RunFrozen lab/kiln cancelled",
                "RoadmapBlocked lab/kiln",
                "RunStarted lab/kiln",
                "ToolResult lab/kiln plan failed",
                "RunFrozen lab/kiln done",
            ]
            .map(str::to_owned)
            .to_vec()
        ),
        "the room delegated again starts under the same lead and cannot take the node its failed child blocked"
    );
}

#[test]
fn a_verification_that_fails_lands_nothing_and_finishes_no_node() {
    let dir = tempfile::tempdir().unwrap();
    let board = Arc::new(Board::default());
    let (mut worker, ledger) = city(dir.path(), Planners(Arc::clone(&board)));
    raise(&mut worker, "lab");
    plan_lab(dir.path());
    let lab = CityLayout::new(dir.path()).scope(&Address::parse("lab").unwrap());

    std::fs::write(
        lab.join(kernel::RESERVED_PREFIX).join("RULES.toml"),
        "confidential = false\nwrite = \"everything\"\nreview = true\n",
    )
    .unwrap();
    let notes = lab.join("kiln").join(NOTES);
    std::fs::create_dir_all(notes.parent().unwrap()).unwrap();
    std::fs::write(&notes, NOTES_BEFORE).unwrap();
    *board.ledger.lock().unwrap() = Some(ledger.clone());

    let offered: Mark = Box::new(|lines: &Lines| {
        quiet(lines)
            && lines
                .iter()
                .any(|(record, _)| record.kind() == EventKind::PrOpened)
    });
    let lines = serve_in_turn(
        worker,
        &ledger,
        vec![
            (Box::new(|_: &Lines| true), dispatch("lab/kiln", MAKER)),
            (offered, dispatch("lab/glaze", CHECKER)),
        ],
        |lines| frozen(lines) == 2 && quiet(lines),
    );
    let review = |line: &(EventRecord, String)| {
        matches!(
            line.0.kind(),
            EventKind::PrOpened | EventKind::PrMerged | EventKind::PrRejected
        )
    };
    assert_eq!(
        (
            plan_trace(&lines, |line| !review(line)),
            lines
                .iter()
                .filter(|line| review(line))
                .map(|(record, _)| format!("{:?}", record.kind()))
                .collect::<Vec<_>>(),
            std::fs::read_to_string(&notes).unwrap(),
        ),
        (
            [
                "RunStarted lab/kiln",
                "RoadmapClaimed lab/kiln",
                "ToolResult lab/kiln plan answered",
                "RunFrozen lab/kiln done",
                "RoadmapBlocked lab/kiln",
                "RunStarted lab/glaze",
                "RunFrozen lab/glaze done",
            ]
            .map(str::to_owned)
            .to_vec(),
            vec!["PrOpened".to_owned(), "PrRejected".to_owned()],
            NOTES_BEFORE.to_owned(),
        ),
        "a failed check merges nothing, and the node its maker held is not finished"
    );
}
