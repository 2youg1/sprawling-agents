// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The lazy credential scan against the full scan it replaces
//! (`crates/accounting/spec/Playback/Project.lean`, last section, D47).
//!
//! The model proves that the lazy scan hands every table the fate the
//! full scan does; these cases hold the Rust to it on random histories
//! and on a fixed narrow export, comparing the whole bundle's bytes and
//! the number of lines each scan read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{EventKind, RunId, Seq};
use proptest::prelude::*;
use serde_json::{Value, json};

use super::super::encode::encode;
use super::super::project::Scanning;
use super::super::{Projected, Request, Selection, project};
use super::{Line, lines, parsed, person, run, write};

/// A value `kernel::secret::scan` reads as a credential.
fn credential() -> String {
    format!("sk-ant-{}", "a1B2c3D4e5".repeat(9))
}

/// What one generated line is.
#[derive(Debug, Clone, Copy)]
enum Op {
    ToolCalled,
    ToolResult,
    ModelCalled,
    ModelReturned,
    Member,
}

/// One generated line: its run, what it is, the key its call pairs by,
/// and whether its payload carries a credential.
type Step = (u8, Op, u8, bool);

fn line_of((which, op, key, secret): Step) -> Line {
    let note = if secret {
        credential()
    } else {
        "plain".to_owned()
    };
    let (kind, data) = match op {
        Op::ToolCalled => (
            EventKind::ToolCalled,
            json!({"id": format!("c{key}"), "name": "read", "note": note}),
        ),
        Op::ToolResult => (
            EventKind::ToolResult,
            json!({"tool_use_id": format!("c{key}"), "note": note}),
        ),
        Op::ModelCalled => (
            EventKind::ModelCalled,
            json!({"model": format!("m{key}"), "note": note}),
        ),
        Op::ModelReturned => (EventKind::ModelReturned, json!({"note": note})),
        Op::Member => (EventKind::ModelSelected, json!({"note": note})),
    };
    (run(which), Some("lab/a"), kind, data)
}

/// The city's opening lines: the city, the open building `lab`, and
/// runs 1 and 2 started in it.
fn opening() -> Vec<Line> {
    vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (
            RunId::CITY,
            Some("lab"),
            EventKind::BuildingCreated,
            json!({"addr": "lab", "template": "builds"}),
        ),
        (run(1), Some("lab/a"), EventKind::RunStarted, json!({})),
        (run(2), Some("lab/a"), EventKind::RunStarted, json!({})),
    ]
}

/// The bundle's bytes and the number of lines scanned, exported from a
/// city holding `steps` after the opening lines.
fn exported(steps: &[Step], selection: &Selection, scanning: Scanning) -> (Vec<u8>, u64) {
    let dir = tempfile::tempdir().unwrap();
    let script = opening()
        .into_iter()
        .chain(steps.iter().copied().map(line_of))
        .collect();
    write(dir.path(), &lines(script), b"");
    let request = Request {
        selection: selection.clone(),
        ..person()
    };
    match project(dir.path(), &request, scanning).unwrap() {
        Projected::Whole { document, scans } => (encode(&document).unwrap().bytes().to_vec(), scans),
        Projected::EndsAt(reached) => panic!("the ledger ended at {reached:?}"),
    }
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::ToolCalled),
        Just(Op::ToolResult),
        Just(Op::ModelCalled),
        Just(Op::ModelReturned),
        Just(Op::Member),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    (1u8..=2, op(), 0u8..3, prop::bool::weighted(0.3))
}

/// A history, and a selection of a seq range in it, perhaps of one run.
fn history() -> impl Strategy<Value = (Vec<Step>, Selection)> {
    prop::collection::vec(step(), 1..24).prop_flat_map(|steps| {
        let last = u64::try_from(steps.len() + 3).unwrap();
        (Just(steps), 0..=last, 0..=last, prop::option::of(1u8..=2)).prop_map(
            |(steps, a, b, which)| {
                let selection = Selection::new(
                    Some(Seq::new(a.min(b))),
                    Some(Seq::new(a.max(b))),
                    which.map(run),
                    None,
                )
                .unwrap();
                (steps, selection)
            },
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn the_lazy_scan_exports_the_bundle_the_full_scan_does((steps, selection) in history()) {
        let (full, full_scans) = exported(&steps, &selection, Scanning::Full);
        let (lazy, lazy_scans) = exported(&steps, &selection, Scanning::Lazy);
        prop_assert_eq!(parsed(&lazy), parsed(&full));
        prop_assert!(lazy_scans <= full_scans, "{lazy_scans} scans against {full_scans}");
    }
}

/// `forgetting_a_pairing_leaks`: a credential on a model call outside
/// the selection, answered inside it, keeps the call's name out.
#[test]
fn an_unselected_call_with_a_credential_keeps_its_name_out() {
    let steps = [
        (1, Op::ModelCalled, 0, true),
        (1, Op::ModelReturned, 0, false),
    ];
    let selection = Selection::new(Some(Seq::new(5)), Some(Seq::new(5)), None, None).unwrap();
    let (full, _) = exported(&steps, &selection, Scanning::Full);
    let (lazy, _) = exported(&steps, &selection, Scanning::Lazy);
    let bundle: Value = parsed(&lazy);
    assert_eq!(bundle["calls"][0]["called"], "withheld");
    assert_eq!(bundle, parsed(&full));
}

/// H10, `a_credential_line_is_withheld`: a selected line with a
/// credential is counted, not shown.
#[test]
fn a_selected_credential_line_is_counted_not_shown() {
    let steps = [(1, Op::ToolCalled, 0, true), (1, Op::ToolResult, 0, false)];
    let selection = Selection::new(Some(Seq::new(4)), Some(Seq::new(5)), None, None).unwrap();
    let (lazy, _) = exported(&steps, &selection, Scanning::Lazy);
    let bundle = parsed(&lazy);
    assert_eq!(bundle["withheld"]["credential"], "1");
    assert_eq!(bundle["calls"][0]["called"], "withheld");
}

/// A narrow export of a long history scans the selected lines, the
/// lines a table reads without pairing, and the far ends of the
/// selected pairs; the full scan reads every line.
#[test]
fn a_narrow_export_scans_only_what_its_tables_read() {
    let steps: Vec<Step> = (0..200u16)
        .flat_map(|round| {
            let key = u8::try_from(round % 3).unwrap();
            [
                (1, Op::ModelCalled, key, false),
                (1, Op::ModelReturned, key, false),
                (1, Op::ToolCalled, key, false),
                (1, Op::ToolResult, key, false),
                (1, Op::Member, key, false),
            ]
        })
        .collect();
    let selection = Selection::new(Some(Seq::new(500)), Some(Seq::new(504)), None, None).unwrap();
    let (full, full_scans) = exported(&steps, &selection, Scanning::Full);
    let (lazy, lazy_scans) = exported(&steps, &selection, Scanning::Lazy);
    assert_eq!(parsed(&lazy), parsed(&full));
    assert_eq!((full_scans, lazy_scans), (1004, 208));
}
