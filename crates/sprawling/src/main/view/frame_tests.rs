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

use super::follow::Row;
use super::frame::{FILLING, Face, Size};
use super::keys::Action;
use kernel::{Address, RunId, Seq};
use sprawling::lineage::RunLine;

fn run(n: u8) -> RunId {
    RunId::parse(&format!("0198f6a2-7c4a-7bbb-9d1e-0000000000{n:02}")).unwrap()
}

fn line(n: u8, addr: &str, seqs: (u64, u64), state: Option<storage::RunPhase>) -> RunLine {
    RunLine {
        run: run(n),
        addr: Some(Address::parse(addr).unwrap()),
        session: None,
        parent: None,
        forked_at: None,
        predecessor: None,
        first_seq: Seq::new(seqs.0),
        last_seq: Seq::new(seqs.1),
        state,
        unanswered: 0,
    }
}

/// Run 1 in lab/a, run 3 forked from it, run 4 taking over from run 1
/// in a second stretch, and run 2, the only active one, in yard/b.
fn city_runs() -> Vec<RunLine> {
    vec![
        line(1, "lab/a", (1, 4), None),
        line(2, "yard/b", (3, 8), Some(storage::RunPhase::Active)),
        RunLine {
            parent: Some(run(1)),
            forked_at: Some(Seq::new(2)),
            ..line(3, "lab/a", (5, 7), Some(storage::RunPhase::Frozen))
        },
        RunLine {
            session: Some(Seq::new(9)),
            predecessor: Some(run(1)),
            ..line(4, "lab/a", (10, 11), None)
        },
    ]
}

fn city(size: Size) -> Face {
    let runs = city_runs();
    let owners = [1, 1, 2, 1, 3, 2, 3, 2, 0, 4, 4];
    let records = (1_u64..)
        .zip(owners)
        .map(|(seq, owner)| {
            let run = if owner == 0 { RunId::CITY } else { run(owner) };
            Row {
                seq: Seq::new(seq),
                run,
                line: format!(r#"{{"seq":{seq},"run":"{run}"}}"#),
            }
        })
        .collect();
    Face::open(&runs, records, size)
}

const NARROW: Size = Size {
    columns: 100,
    rows: 10,
};

fn after(actions: &[Action], size: Size) -> Face {
    let mut face = city(size);
    for action in actions {
        face.apply(*action);
    }
    face
}

fn cursor_line(face: &Face) -> String {
    face.frame()
        .into_iter()
        .find(|line| line.starts_with('>'))
        .unwrap_or_default()
}

const R1: &str = "0198f6a2-7c4a-7bbb-9d1e-000000000001";
const R2: &str = "0198f6a2-7c4a-7bbb-9d1e-000000000002";
const R3: &str = "0198f6a2-7c4a-7bbb-9d1e-000000000003";
const R4: &str = "0198f6a2-7c4a-7bbb-9d1e-000000000004";

/// A frozen run still waiting on an answer outranks a younger active run.
#[test]
fn opens_on_the_run_waiting_for_the_person_before_any_active_run() {
    let runs = vec![
        RunLine {
            unanswered: 1,
            ..line(3, "lab/a", (1, 2), Some(storage::RunPhase::Frozen))
        },
        line(2, "yard/b", (3, 8), Some(storage::RunPhase::Active)),
    ];
    let face = Face::open(&runs, Vec::new(), NARROW);
    assert_eq!(
        cursor_line(&face),
        format!(">        + run {R3} frozen #1..2")
    );
}

/// A run that starts while the viewer is open appears with its
/// ancestors unfolded, and the cursor and folds the person left stay.
#[test]
fn a_run_that_starts_while_following_appears_and_the_cursor_stays() {
    let mut face = city(NARROW);
    let runs = vec![
        line(1, "lab/a", (1, 4), None),
        line(2, "yard/b", (3, 8), Some(storage::RunPhase::Active)),
        RunLine {
            parent: Some(run(1)),
            forked_at: Some(Seq::new(2)),
            ..line(3, "lab/a", (5, 7), Some(storage::RunPhase::Frozen))
        },
        RunLine {
            session: Some(Seq::new(9)),
            predecessor: Some(run(1)),
            ..line(4, "lab/a", (10, 11), None)
        },
        line(5, "lab/c", (12, 12), Some(storage::RunPhase::Active)),
    ];
    let appended = vec![Row {
        seq: Seq::new(12),
        run: run(5),
        line: "{}".to_owned(),
    }];
    face.follow(&runs, appended);
    assert_eq!(
        face.frame(),
        vec![
            " - city".to_owned(),
            "   - lab".to_owned(),
            "     + lab/a".to_owned(),
            "     - lab/c".to_owned(),
            "       - session (first stretch)".to_owned(),
            "         + run 0198f6a2-7c4a-7bbb-9d1e-000000000005 active #12..12".to_owned(),
            "   - yard".to_owned(),
            "     - yard/b".to_owned(),
            "       - session (first stretch)".to_owned(),
            format!(">        + run {R2} active #3..8"),
        ]
    );
    face.apply(Action::SwitchLens);
    face.apply(Action::Last);
    assert_eq!(cursor_line(&face), ">{}");
}

#[test]
fn opens_on_the_latest_active_run_with_only_its_ancestors_open() {
    assert_eq!(
        city(NARROW).frame(),
        vec![
            " - city".to_owned(),
            "   + lab".to_owned(),
            "   - yard".to_owned(),
            "     - yard/b".to_owned(),
            "       - session (first stretch)".to_owned(),
            format!(">        + run {R2} active #3..8"),
        ]
    );
}

#[test]
fn every_movement_lands_on_the_row_it_names() {
    let session = ">      - session (first stretch)".to_owned();
    assert_eq!(cursor_line(&after(&[Action::Up], NARROW)), session);
    assert_eq!(
        cursor_line(&after(&[Action::Up, Action::Down, Action::Down], NARROW)),
        format!(">        + run {R2} active #3..8")
    );
    assert_eq!(cursor_line(&after(&[Action::First], NARROW)), ">- city");
    assert_eq!(
        cursor_line(&after(&[Action::First, Action::Last], NARROW)),
        format!(">        + run {R2} active #3..8")
    );
    let short = Size {
        columns: 100,
        rows: 3,
    };
    assert_eq!(cursor_line(&after(&[Action::PageUp], short)), ">  - yard");
    assert_eq!(
        cursor_line(&after(&[Action::PageUp, Action::PageDown], short)),
        format!(">        + run {R2} active #3..8")
    );
}

#[test]
fn collapse_climbs_then_folds_and_expand_unfolds_then_descends() {
    let folded = after(&[Action::Collapse, Action::Collapse], NARROW);
    assert_eq!(cursor_line(&folded), ">      + session (first stretch)");
    assert_eq!(folded.frame().len(), 5);
    let lab = after(&[Action::First, Action::Down, Action::Expand], NARROW);
    assert_eq!(cursor_line(&lab), ">  - lab");
    assert!(lab.frame().contains(&"     + lab/a".to_owned()));
    let room = after(
        &[Action::First, Action::Down, Action::Expand, Action::Expand],
        NARROW,
    );
    assert_eq!(cursor_line(&room), ">    + lab/a");
}

#[test]
fn switching_lens_keeps_the_selected_thing() {
    let records = after(&[Action::SwitchLens], NARROW);
    assert_eq!(
        cursor_line(&records),
        format!(r#">{{"seq":3,"run":"{R2}"}}"#)
    );
    let back = after(
        &[Action::SwitchLens, Action::Down, Action::SwitchLens],
        NARROW,
    );
    assert_eq!(
        cursor_line(&back),
        format!(">        + run {R1} ended #1..4")
    );
}

#[test]
fn detail_takes_the_screen_until_closed_and_quit_closes_the_viewer() {
    let full = after(&[Action::OpenDetail], NARROW);
    assert!(full.frame().contains(&r#"addr: "yard/b""#.to_owned()));
    assert!(!full.frame().iter().any(|line| line.starts_with('>')));
    assert_eq!(
        after(&[Action::OpenDetail, Action::CloseDetail], NARROW).frame(),
        city(NARROW).frame()
    );
    assert!(!city(NARROW).is_closed());
    assert!(after(&[Action::Quit], NARROW).is_closed());
}

#[test]
fn a_wide_terminal_keeps_the_detail_pane_open_on_the_right() {
    let wide = city(Size {
        columns: 120,
        rows: 10,
    })
    .frame();
    assert_eq!(wide.len(), 10);
    assert!(wide.iter().all(|line| line.chars().nth(60) == Some('|')));
    assert!(wide.iter().any(|line| line.ends_with(r#"|addr: "yard/b""#)));
}

fn ledger_row(seq: u64, kind: wire::EventKind, data: serde_json::Value) -> Row {
    let record = wire::EventRecord::from_draft(
        wire::EventDraft {
            run: run(2),
            t: wire::TimeMs::new(seq),
            who: "yard/b".to_owned(),
            addr: None,
            kind,
            data: wire::Payload::new(data.as_object().unwrap().clone()).unwrap(),
            ig: false,
        },
        Seq::new(seq),
        wire::B3Hash::digest(b"prev"),
    );
    Row {
        seq: Seq::new(seq),
        run: run(2),
        line: String::from_utf8(record.canonical_line().unwrap()).unwrap(),
    }
}

/// Opening a run folds its lines into rounds, and each round holds the
/// calls made in it.
#[test]
fn expanding_a_run_shows_its_rounds_and_their_calls() {
    let runs = vec![line(2, "yard/b", (3, 5), Some(storage::RunPhase::Active))];
    let records = vec![
        ledger_row(3, wire::EventKind::ModelCalled, serde_json::json!({})),
        ledger_row(
            4,
            wire::EventKind::ToolCalled,
            serde_json::json!({ "id": "c1", "name": "read", "subject": "a.rs" }),
        ),
        ledger_row(5, wire::EventKind::ModelCalled, serde_json::json!({})),
    ];
    let mut face = Face::open(&runs, records, NARROW);
    for action in [Action::Expand, Action::Down, Action::Expand] {
        face.apply(action);
    }
    assert_eq!(
        face.frame()[4..],
        [
            format!("         - run {R2} active #3..5"),
            ">          - round 1 @3".to_owned(),
            "               read a.rs".to_owned(),
            "             round 2 @5".to_owned(),
        ]
    );
}

/// A run is marked openable until its rounds are folded: whether it has
/// any is not known before the person first opens it.
#[test]
fn a_run_whose_rounds_are_not_folded_yet_is_marked_openable() {
    let runs = vec![line(2, "yard/b", (3, 3), Some(storage::RunPhase::Active))];
    let records = vec![ledger_row(
        3,
        wire::EventKind::ModelCalled,
        serde_json::json!({}),
    )];
    assert_eq!(
        Face::open(&runs, records, NARROW).frame()[4],
        format!(">        + run {R2} active #3..3")
    );
}

/// A first screen read from the tail says the older lines are being
/// filled; the whole fold places them before the window and gives a
/// run whose `run_started` lay outside it its address.
#[test]
fn a_window_from_the_tail_says_it_is_filling_until_the_whole_fold_arrives() {
    let windowed = RunLine {
        addr: None,
        ..line(4, "lab/a", (11, 11), Some(storage::RunPhase::Active))
    };
    let rows = |from: u64| -> Vec<Row> {
        (from..=11)
            .map(|seq| Row {
                seq: Seq::new(seq),
                run: run(4),
                line: format!(r#"{{"seq":{seq}}}"#),
            })
            .collect()
    };
    let mut face = Face::open_window(&[windowed], rows(11), NARROW);
    let first = face.frame();
    assert_eq!(
        (first.len(), first.last().map(String::as_str)),
        (NARROW.rows, Some(FILLING))
    );
    face.fill(&city_runs(), rows(1));
    face.apply(Action::SwitchLens);
    face.apply(Action::First);
    let oldest = cursor_line(&face);
    face.apply(Action::SwitchLens);
    let run_line = cursor_line(&face);
    assert_eq!(
        (
            oldest,
            run_line.starts_with(&format!(">        + run {R4}")),
            face.frame().contains(&FILLING.to_owned())
        ),
        (r#">{"seq":1}"#.to_owned(), true, false)
    );
}
