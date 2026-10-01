// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The scenes the Lean model of selection states, run through the
//! production export (accounting-SPEC.md 8-12, decision 25(i)).
//!
//! `crates/accounting/spec/Playback/Select.lean` proves that its model
//! selects each scene's seqs from its ledger. This test reads the same
//! ledger and the same scenes out of that file, line by line, writes the
//! ledger, and asks the export for each scene. A behaviour comparison,
//! not a proof that the Rust refines the model.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::collections::BTreeSet;

use kernel::{EventKind, RunId, Seq};
use serde_json::json;

use super::super::{Confidential, Cutoff, Reader, Request, Selection, export};
use super::{addr, lines, parsed, run, seqs, write};

/// The model, read as the text it is.
const MODEL: &str = include_str!("../../../spec/Playback/Select.lean");

/// One scene: the four conditions, the cutoff, and the seqs the model
/// selects.
#[derive(Debug)]
struct Scene {
    conditions: [Option<u64>; 4],
    cutoff: u64,
    seqs: Vec<String>,
}

/// The words of a table line, its brackets, parentheses and commas
/// dropped.
fn words(line: &str) -> Vec<&str> {
    line.split(|c: char| c.is_whitespace() || "(),[]".contains(c))
        .filter(|word| !word.is_empty())
        .collect()
}

/// `some n` or `none`, taken from the front of `words`.
fn option(words: &mut std::slice::Iter<'_, &str>) -> Option<u64> {
    match words.next() {
        Some(&"none") => None,
        Some(&"some") => Some(words.next().unwrap().parse().unwrap()),
        other => panic!("{other:?} is neither `some n` nor `none`"),
    }
}

/// The model's ledger: `(seq, run, building)` per `line` row.
fn ledger() -> Vec<(u64, u64, Option<u64>)> {
    MODEL
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("line "))
        .map(|line| {
            let words = words(line);
            let mut rest = words[3..].iter();
            (
                words[1].parse().unwrap(),
                words[2].parse().unwrap(),
                option(&mut rest),
            )
        })
        .collect()
}

/// The model's scenes, one per `scene` row.
fn scenes() -> Vec<Scene> {
    MODEL
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("scene "))
        .map(|line| {
            let words = words(line);
            let mut rest = words[1..].iter();
            let conditions = [
                option(&mut rest),
                option(&mut rest),
                option(&mut rest),
                option(&mut rest),
            ];
            let cutoff = rest.next().unwrap().parse().unwrap();
            Scene {
                conditions,
                cutoff,
                seqs: rest.map(|seq| (*seq).to_owned()).collect(),
            }
        })
        .collect()
}

/// The model's run `n`: run 0 is the city's own.
fn run_of(n: u64) -> RunId {
    if n == 0 {
        RunId::CITY
    } else {
        run(u8::try_from(n).unwrap())
    }
}

/// The room a line of building `b` is addressed in.
fn room_of(building: u64) -> &'static str {
    match building {
        0 => "b0/room",
        1 => "b1/room",
        other => panic!("the scenes name building {other}, which this test does not spell"),
    }
}

#[test]
fn every_scene_of_the_selection_model_selects_the_same_seqs_in_production() {
    let dir = tempfile::tempdir().unwrap();
    let mut started = BTreeSet::new();
    let script = ledger()
        .into_iter()
        .enumerate()
        .map(|(at, (seq, n, building))| {
            assert_eq!(
                u64::try_from(at).unwrap(),
                seq,
                "the model's ledger skips a seq"
            );
            let kind = if seq == 0 {
                EventKind::CityInitialized
            } else if started.insert(n) {
                EventKind::RunStarted
            } else {
                EventKind::ToolCalled
            };
            (
                run_of(n),
                building.map(room_of),
                kind,
                json!({"tool": "read"}),
            )
        })
        .collect();
    write(dir.path(), &lines(script), b"");
    let scenes = scenes();
    assert!(scenes.len() >= 16, "only {} scenes were read", scenes.len());
    let selected: Vec<(usize, Vec<String>)> = scenes
        .iter()
        .enumerate()
        .map(|(at, scene)| {
            let [first, last, run, building] = scene.conditions;
            let request = Request {
                selection: Selection::new(
                    first.map(Seq::new),
                    last.map(Seq::new),
                    run.map(run_of),
                    building.map(|b| addr(&format!("b{b}"))),
                )
                .unwrap(),
                reader: Reader::Person(Confidential::Withheld),
                cutoff: Cutoff::At(Seq::new(scene.cutoff)),
            };
            let bundle = parsed(export(dir.path(), &request).unwrap().bytes());
            (at, seqs(&bundle["events"]))
        })
        .collect();
    let expected: Vec<(usize, Vec<String>)> = scenes
        .into_iter()
        .enumerate()
        .map(|(at, scene)| (at, scene.seqs))
        .collect();
    assert_eq!(selected, expected);
}
