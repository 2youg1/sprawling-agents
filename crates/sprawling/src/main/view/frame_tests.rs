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

use super::frame::{Face, Row, Size};
use super::keys::{Action, Key, action_for};
use kernel::{Address, RunId, Seq};
use sprawling::lineage::RunLine;

fn run(n: u8) -> RunId {
    RunId::parse(&format!("0198f6a2-7c4a-7bbb-9d1e-0000000000{n:02}")).unwrap()
}

fn line(n: u8, addr: &str, seqs: (u64, u64), state: Option<memory::RunPhase>) -> RunLine {
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
    }
}

/// Run 1 in lab/a, run 3 forked from it, run 4 taking over from run 1
/// in a second stretch, and run 2, the only active one, in yard/b.
fn city(size: Size) -> Face {
    let runs = vec![
        line(1, "lab/a", (1, 4), None),
        line(2, "yard/b", (3, 8), Some(memory::RunPhase::Active)),
        RunLine {
            parent: Some(run(1)),
            forked_at: Some(Seq::new(2)),
            ..line(3, "lab/a", (5, 7), Some(memory::RunPhase::Frozen))
        },
        RunLine {
            session: Some(Seq::new(9)),
            predecessor: Some(run(1)),
            ..line(4, "lab/a", (10, 11), None)
        },
    ];
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
            format!(">          run {R2} active #3..8"),
        ]
    );
}

#[test]
fn every_movement_lands_on_the_row_it_names() {
    let session = ">      - session (first stretch)".to_owned();
    assert_eq!(cursor_line(&after(&[Action::Up], NARROW)), session);
    assert_eq!(
        cursor_line(&after(&[Action::Up, Action::Down, Action::Down], NARROW)),
        format!(">          run {R2} active #3..8")
    );
    assert_eq!(cursor_line(&after(&[Action::First], NARROW)), ">- city");
    assert_eq!(
        cursor_line(&after(&[Action::First, Action::Last], NARROW)),
        format!(">          run {R2} active #3..8")
    );
    let short = Size {
        columns: 100,
        rows: 3,
    };
    assert_eq!(cursor_line(&after(&[Action::PageUp], short)), ">  - yard");
    assert_eq!(
        cursor_line(&after(&[Action::PageUp, Action::PageDown], short)),
        format!(">          run {R2} active #3..8")
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

#[test]
fn every_key_the_viewer_reads_names_its_action() {
    let table = [
        (Key::Char('k'), Some(Action::Up)),
        (Key::Up, Some(Action::Up)),
        (Key::Char('j'), Some(Action::Down)),
        (Key::Down, Some(Action::Down)),
        (Key::PageUp, Some(Action::PageUp)),
        (Key::PageDown, Some(Action::PageDown)),
        (Key::Char('g'), Some(Action::First)),
        (Key::Home, Some(Action::First)),
        (Key::Char('G'), Some(Action::Last)),
        (Key::End, Some(Action::Last)),
        (Key::Char('h'), Some(Action::Collapse)),
        (Key::Left, Some(Action::Collapse)),
        (Key::Char('l'), Some(Action::Expand)),
        (Key::Right, Some(Action::Expand)),
        (Key::Tab, Some(Action::SwitchLens)),
        (Key::Enter, Some(Action::OpenDetail)),
        (Key::Esc, Some(Action::CloseDetail)),
        (Key::Char('q'), Some(Action::Quit)),
        (Key::Interrupt, Some(Action::Quit)),
        (Key::Char('x'), None),
    ];
    for (key, action) in table {
        assert_eq!(action_for(key), action, "{key:?}");
    }
}
