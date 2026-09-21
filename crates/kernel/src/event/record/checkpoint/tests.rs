// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes these variants write are the bytes
//! `memory::checkpoint::fence::committed` and
//! `runtime::run::lifecycle::dispatch` wrote by hand, including the
//! `effort` word an unasked effort was recorded under.

use serde_json::{Map, Value};

use super::*;
use crate::event::Payload;

const OID: &str = "0123456789abcdef0123456789abcdef01234567";

fn bytes(payload: &Payload) -> String {
    serde_json::to_string(payload).unwrap()
}

fn oid() -> GitOid {
    GitOid::parse(OID).unwrap()
}

/// `Provenance::model_fields` followed by `committed`, copied key for
/// key from the build that wrote the ledgers this city already holds.
fn hand_written_commit(effort_word: &str, predecessor: Option<RunId>, scope: &[&str]) -> Payload {
    let mut map = Map::new();
    map.insert(
        "model".to_owned(),
        Value::String("claude-sonnet-4-6".to_owned()),
    );
    map.insert("effort".to_owned(), Value::String(effort_word.to_owned()));
    if let Some(predecessor) = predecessor {
        map.insert(
            "predecessor".to_owned(),
            Value::String(predecessor.to_string()),
        );
    }
    map.insert("oid".to_owned(), Value::String(OID.to_owned()));
    map.insert(
        "scope".to_owned(),
        Value::Array(
            scope
                .iter()
                .map(|at| Value::String((*at).to_owned()))
                .collect(),
        ),
    );
    map.insert(
        "files".to_owned(),
        Value::Array(vec![Value::String("lab/notes.md".to_owned())]),
    );
    Payload::new(map).unwrap()
}

fn commit(effort: Option<Effort>, predecessor: Option<RunId>, scope: &[&str]) -> Commit {
    Commit {
        oid: oid(),
        by: CommitAttribution {
            model: "claude-sonnet-4-6".to_owned(),
            effort,
            predecessor,
        },
        scope: scope.iter().map(|at| (*at).to_owned()).collect(),
        files: vec!["lab/notes.md".to_owned()],
    }
}

#[test]
fn a_fence_writes_the_bytes_the_hand_written_map_wrote() {
    let old = hand_written_commit("high", None, &["lab"]);
    let new = Payload::of(&CheckpointCommitted::Committed(commit(
        Some(Effort::High),
        None,
        &["lab"],
    )))
    .unwrap();
    assert_eq!(bytes(&new), bytes(&old));
}

#[test]
fn an_unasked_effort_is_still_written_as_the_word_none() {
    // `effort_word(None)` produced "none", and so does `Effort::None`.
    // The two facts have been one word on this ledger since the first
    // fence, and this test is what keeps that true through the change.
    let old = hand_written_commit("none", None, &[]);
    let new = Payload::of(&CheckpointCommitted::Committed(commit(
        Some(Effort::None),
        None,
        &[],
    )))
    .unwrap();
    assert_eq!(bytes(&new), bytes(&old));
    assert!(bytes(&new).contains(r#""scope":[]"#), "{}", bytes(&new));
}

#[test]
fn a_successor_adds_the_predecessor_key_and_a_first_run_omits_it() {
    let predecessor = RunId::from_bytes([3u8; 16]);
    let old = hand_written_commit("low", Some(predecessor), &["lab", "yard"]);
    let new = Payload::of(&CheckpointCommitted::Committed(commit(
        Some(Effort::Low),
        Some(predecessor),
        &["lab", "yard"],
    )))
    .unwrap();
    assert_eq!(bytes(&new), bytes(&old));
    assert!(
        !bytes(
            &Payload::of(&CheckpointCommitted::Committed(commit(
                Some(Effort::Low),
                None,
                &[]
            )))
            .unwrap()
        )
        .contains("predecessor")
    );
}

#[test]
fn the_dispatch_pin_writes_only_its_job_and_reads_back_as_a_pin() {
    let raw = r#"{"job":"file:sim/lobby/room1@8ea61adc1c8339d02a5c665ba8280a85151ee549"}"#;
    let job =
        Locator::parse("file:sim/lobby/room1@8ea61adc1c8339d02a5c665ba8280a85151ee549").unwrap();
    let pinned = CheckpointCommitted::JobPinned { job: job.clone() };
    assert_eq!(bytes(&Payload::of(&pinned).unwrap()), raw);
    let payload: Payload = serde_json::from_str(raw).unwrap();
    assert_eq!(payload.read::<CheckpointCommitted>().unwrap(), pinned);
}

#[test]
fn a_commit_line_reads_as_a_commit_and_never_as_a_pin() {
    let old = hand_written_commit("high", None, &["lab"]);
    let read: CheckpointCommitted = old.read().unwrap();
    assert_eq!(
        read,
        CheckpointCommitted::Committed(commit(Some(Effort::High), None, &["lab"]))
    );
}

#[test]
fn a_record_that_named_no_model_reads_as_saying_nothing_about_one() {
    let mut map = Map::new();
    map.insert("oid".to_owned(), Value::String(OID.to_owned()));
    let read: CheckpointCommitted = Payload::new(map).unwrap().read().unwrap();
    let CheckpointCommitted::Committed(held) = read else {
        panic!("a line with an oid is a commit");
    };
    assert_eq!(held.by, CommitAttribution::default());
    assert!(held.scope.is_empty());
}

#[test]
fn a_line_that_names_neither_a_job_nor_a_commit_is_refused() {
    // A reader that accepted `{}` here would report a checkpoint with
    // an invented oid; the empty payload the test fixtures carry is a
    // `run_started`, which has its own struct.
    assert!(Payload::empty().read::<CheckpointCommitted>().is_err());
}
