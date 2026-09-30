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

use accounting::playback::{Report, Verdict};
use kernel::{AxCode, B3Hash};
use serde_json::json;

use super::super::exit::Exit;
use super::super::grammar::{Invocation, parse};
use super::super::verbs::Verb;
use super::{Refused, outcome, request_of, write_new};

fn listed(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_bundle_lands_whole_and_leaves_no_staged_copy() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("day.json");
    let landed = write_new(&target, b"{}").map_err(|err| *err.code());
    assert_eq!(
        (landed, std::fs::read(&target).ok(), listed(dir.path())),
        (Ok(()), Some(b"{}".to_vec()), vec!["day.json".to_owned()])
    );
}

#[test]
fn a_bundle_never_overwrites_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("day.json");
    std::fs::write(&target, b"kept").unwrap();
    let landed = write_new(&target, b"{}").map_err(|err| *err.code());
    assert_eq!(
        (landed, std::fs::read(&target).unwrap(), listed(dir.path())),
        (
            Err(AxCode::InvalidArgs),
            b"kept".to_vec(),
            vec!["day.json".to_owned()]
        )
    );
}

#[test]
fn a_bundle_never_lands_in_protected_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = dir.path().join(".sprawling");
    let git = dir.path().join(".GIT");
    std::fs::create_dir_all(&ledger).unwrap();
    std::fs::create_dir_all(&git).unwrap();
    let into_ledger = write_new(&ledger.join("day.json"), b"{}").map_err(|err| *err.code());
    let into_git = write_new(&git.join("day.json"), b"{}").map_err(|err| *err.code());
    assert_eq!(
        (into_ledger, into_git, listed(&ledger), listed(&git)),
        (
            Err(AxCode::OutsideWriteDomain),
            Err(AxCode::OutsideWriteDomain),
            Vec::<String>::new(),
            Vec::<String>::new()
        )
    );
}

/// How `playback export` refuses `line`, the verb given as the one word
/// its row is named, so the grammar's two-word reading is not what this
/// judges.
fn refusal_of(line: &[&str]) -> Option<&'static str> {
    let words: Vec<String> = line.iter().map(|word| (*word).to_owned()).collect();
    let Ok(Invocation::Run(Verb::PlaybackExport, read)) = parse(&words) else {
        panic!("{line:?} did not parse as playback export");
    };
    match request_of(&read) {
        Ok(_) => None,
        Err(Refused::Line(_)) => Some("line"),
        Err(Refused::Selection(_)) => Some("selection"),
    }
}

#[test]
fn a_range_or_a_building_that_does_not_read_is_a_command_line_refusal() {
    assert_eq!(
        [
            refusal_of(&["playback export", "c", "--from", "5", "--through", "2"]),
            refusal_of(&["playback export", "c", "--building", "a:b"]),
            refusal_of(&["playback export", "c", "--from", "two"]),
            refusal_of(&["playback export", "c", "--from", "2", "--through", "5"]),
        ],
        [Some("selection"), Some("line"), Some("line"), None]
    );
}

#[test]
fn a_check_that_differs_says_where_and_exits_refused() {
    let digest = B3Hash::digest(b"bundle");
    let found = |verdict| Report {
        digest,
        events: 3,
        verdict,
    };
    assert_eq!(
        [
            outcome(&found(Verdict::Differs { section: "costs" })),
            outcome(&found(Verdict::Same)),
        ],
        [
            (
                json!({"digest": digest.to_string(), "events": "3", "section": "costs", "verdict": "differs"}),
                Exit::Refused
            ),
            (
                json!({"digest": digest.to_string(), "events": "3", "verdict": "same"}),
                Exit::Done
            ),
        ]
    );
}
