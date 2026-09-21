// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes these structs write are the bytes the hand-written maps in
//! `runtime::run::lifecycle` and `runtime::fork` wrote. Each test keeps
//! the old `insert` sequence verbatim and asserts the two encodings are
//! the same string, so a replay over a ledger written by the previous
//! build reads back and re-hashes identically.

use serde_json::{Map, Value};

use super::*;
use crate::event::Payload;

fn bytes(payload: &Payload) -> String {
    serde_json::to_string(payload).unwrap()
}

fn job() -> Locator {
    Locator::parse("file:sim/lobby/room1@8ea61adc1c8339d02a5c665ba8280a85151ee549").unwrap()
}

/// The writer that stood in `runtime::run::lifecycle::dispatch` before
/// this struct existed, copied key for key.
fn hand_written_started(
    task: &str,
    goal: &str,
    parent: Option<RunId>,
    predecessor: Option<RunId>,
) -> Payload {
    let mut started = Map::new();
    started.insert("task".to_owned(), Value::String(task.to_owned()));
    started.insert("goal".to_owned(), Value::String(goal.to_owned()));
    started.insert("job".to_owned(), Value::String(job().to_string()));
    if let Some(parent) = parent {
        started.insert("parent".to_owned(), Value::String(parent.to_string()));
    }
    if let Some(predecessor) = predecessor {
        started.insert(
            "predecessor".to_owned(),
            Value::String(predecessor.to_string()),
        );
    }
    started.insert(
        "skills".to_owned(),
        Value::Array(
            [
                ("read", B3Hash::digest(b"read")),
                ("edit", B3Hash::digest(b"edit")),
            ]
            .into_iter()
            .map(|(name, hash)| {
                let mut row = Map::new();
                row.insert("name".to_owned(), Value::String(name.to_owned()));
                row.insert("hash".to_owned(), Value::String(hash.to_string()));
                Value::Object(row)
            })
            .collect(),
        ),
    );
    Payload::new(started).unwrap()
}

fn typed_started(parent: Option<RunId>, predecessor: Option<RunId>) -> RunStarted {
    RunStarted {
        task: "walk two waves".to_owned(),
        goal: "let an intervention land between them".to_owned(),
        job: Some(job()),
        parent,
        predecessor,
        skills: vec![
            SkillPin {
                name: "read".to_owned(),
                hash: B3Hash::digest(b"read"),
            },
            SkillPin {
                name: "edit".to_owned(),
                hash: B3Hash::digest(b"edit"),
            },
        ],
    }
}

#[test]
fn a_bare_dispatch_writes_the_bytes_the_hand_written_map_wrote() {
    let old = hand_written_started(
        "walk two waves",
        "let an intervention land between them",
        None,
        None,
    );
    let new = Payload::of(&typed_started(None, None)).unwrap();
    assert_eq!(bytes(&new), bytes(&old));
    assert!(!bytes(&new).contains("parent"), "{}", bytes(&new));
    assert!(!bytes(&new).contains("predecessor"), "{}", bytes(&new));
}

#[test]
fn a_fork_and_a_succession_add_exactly_their_own_key() {
    let parent = RunId::from_bytes([7u8; 16]);
    let predecessor = RunId::from_bytes([9u8; 16]);
    for (parent, predecessor) in [
        (Some(parent), None),
        (None, Some(predecessor)),
        (Some(parent), Some(predecessor)),
    ] {
        let old = hand_written_started(
            "walk two waves",
            "let an intervention land between them",
            parent,
            predecessor,
        );
        let new = Payload::of(&typed_started(parent, predecessor)).unwrap();
        assert_eq!(bytes(&new), bytes(&old));
    }
}

#[test]
fn the_golden_fixture_line_reads_back_and_writes_itself_again() {
    let raw = r#"{"goal":"let an intervention land between them","job":"file:sim/lobby/room1@8ea61adc1c8339d02a5c665ba8280a85151ee549","skills":[],"task":"walk two waves"}"#;
    let payload: Payload = serde_json::from_str(raw).unwrap();
    let read: RunStarted = payload.read().unwrap();
    assert_eq!(read.task, "walk two waves");
    assert_eq!(read.job, Some(job()));
    assert!(read.skills.is_empty());
    assert_eq!(bytes(&Payload::of(&read).unwrap()), raw);
}

#[test]
fn an_empty_payload_reads_as_a_run_that_said_nothing() {
    // `fixtures/golden-s1` holds exactly this line. A reader that
    // refused it would refuse a ledger this repository ships.
    let read: RunStarted = Payload::empty().read().unwrap();
    assert_eq!(read, RunStarted::default());
    assert_eq!(read.job, None);
}

#[test]
fn an_unknown_key_passes_so_a_newer_writer_does_not_break_this_reader() {
    let payload: Payload = serde_json::from_str(r#"{"task":"t","mood":"brisk"}"#).unwrap();
    let read: RunStarted = payload.read().unwrap();
    assert_eq!(read.task, "t");
}

#[test]
fn a_fork_writes_the_two_keys_runtime_fork_wrote() {
    let from = RunId::from_bytes([3u8; 16]);
    let mut map = Map::new();
    map.insert("from".to_owned(), Value::String(from.to_string()));
    map.insert("at_seq".to_owned(), Value::from(Seq::new(41).value()));
    let old = Payload::new(map).unwrap();
    let new = Payload::of(&RunForked {
        from,
        at_seq: Seq::new(41),
    })
    .unwrap();
    assert_eq!(bytes(&new), bytes(&old));
    let back: RunForked = new.read().unwrap();
    assert_eq!(back.from, from);
    assert_eq!(back.at_seq, Seq::new(41));
}
