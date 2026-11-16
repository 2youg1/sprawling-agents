// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::collections::BTreeSet;

use kernel::{EventKind, RunId, Seq, TimeMs};
use runtime::clock::UtcSpan;
use serde_json::json;

use super::super::{Confidential, Cutoff, Reader, Request, Selection, export};
use super::{addr, lines_at, parsed, run, seqs, write};

const MODEL: &str = include_str!("../../../spec/Playback/Select.lean");

#[derive(Debug)]
struct Scene {
    conditions: [Option<u64>; 6],
    cutoff: u64,
    seqs: Vec<String>,
}

fn words(line: &str) -> Vec<&str> {
    line.split(|c: char| c.is_whitespace() || "(),[]".contains(c))
        .filter(|word| !word.is_empty())
        .collect()
}

fn option(words: &mut std::slice::Iter<'_, &str>) -> Option<u64> {
    match words.next() {
        Some(&"none") => None,
        Some(&"some") => Some(words.next().unwrap().parse().unwrap()),
        other => panic!("{other:?} is neither `some n` nor `none`"),
    }
}

fn ledger() -> Vec<(u64, u64, Option<u64>, u64)> {
    MODEL
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("line "))
        .map(|line| {
            let words = words(line);
            let mut rest = words[3..].iter();
            let building = option(&mut rest);
            (
                words[1].parse().unwrap(),
                words[2].parse().unwrap(),
                building,
                rest.next().unwrap().parse().unwrap(),
            )
        })
        .collect()
}

fn scenes() -> Vec<Scene> {
    MODEL
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("scene ") || line.starts_with("window "))
        .map(|line| {
            let words = words(line);
            let mut rest = words[1..].iter();
            let conditions = if words[0] == "scene" {
                [
                    option(&mut rest),
                    option(&mut rest),
                    option(&mut rest),
                    option(&mut rest),
                    None,
                    None,
                ]
            } else {
                [None, None, None, None, option(&mut rest), option(&mut rest)]
            };
            let cutoff = rest.next().unwrap().parse().unwrap();
            Scene {
                conditions,
                cutoff,
                seqs: rest.map(|seq| (*seq).to_owned()).collect(),
            }
        })
        .collect()
}

fn run_of(n: u64) -> RunId {
    if n == 0 {
        RunId::CITY
    } else {
        run(u8::try_from(n).unwrap())
    }
}

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
        .map(|(at, (seq, n, building, t))| {
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
                t,
                (
                    run_of(n),
                    building.map(room_of),
                    kind,
                    json!({"tool": "read"}),
                ),
            )
        })
        .collect();
    write(dir.path(), &lines_at(script), b"");
    let scenes = scenes();
    assert!(scenes.len() >= 22, "only {} scenes were read", scenes.len());
    let selected: Vec<(usize, Vec<String>)> = scenes
        .iter()
        .enumerate()
        .map(|(at, scene)| {
            let [first, last, run, building, since, before] = scene.conditions;
            let request = Request {
                selection: Selection::new(
                    first.map(Seq::new),
                    last.map(Seq::new),
                    run.map(run_of),
                    building.map(|b| addr(&format!("b{b}"))),
                )
                .unwrap()
                .during(UtcSpan::new(since.map(TimeMs::new), before.map(TimeMs::new)).unwrap()),
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
