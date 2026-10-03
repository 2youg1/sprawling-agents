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

//! How a session opened and closed, as the rounds answer it: the lines
//! around the turns, and the frozen facts the first of them names.

use kernel::Ledger;
use wire::{B3Hash, EventKind, Payload, RunId, TimeMs};

/// The bytes a session froze for a Mayor called Cat, in the encoding
/// `city::Naming` writes into the store.
const NAMED_CAT: &[u8] = br#"{"person":null,"imported_from":null,"about":"","mayor":"Cat"}"#;

/// Writes one run's lines into a fresh city and answers its rounds.
fn rounds_of(
    dir: &std::path::Path,
    run: RunId,
    started: serde_json::Value,
    stored: &[&[u8]],
) -> wire::RoundsAnswer {
    let report = crate::worker::fixture::init_city(dir).unwrap();
    let mut store = storage::Cas::open(&kernel::layout::CityLayout::new(dir).cas()).unwrap();
    for bytes in stored {
        store.put(bytes).unwrap();
    }
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(9))
        .unwrap()
        .0;
    let drafts = [
        (EventKind::RunStarted, started, TimeMs::new(10)),
        (
            EventKind::WorktreeOpened,
            serde_json::json!({ "name": "hall-mayor", "disk_bytes": 4096 }),
            TimeMs::new(10),
        ),
        (
            EventKind::ModelCalled,
            serde_json::json!({ "segments": [], "model": "m" }),
            TimeMs::new(11),
        ),
        (
            EventKind::RunFrozen,
            serde_json::json!({ "completion": "done", "evidence": [] }),
            TimeMs::new(12),
        ),
    ];
    for (kind, data, t) in drafts {
        ledger
            .append(kernel::EventDraft {
                run,
                t,
                who: "city".to_owned(),
                addr: Some(kernel::Address::parse("hall/mayor").unwrap()),
                kind,
                data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
                ig: false,
            })
            .unwrap();
    }
    drop(ledger);
    let mut views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();
    let wire::Answer::Rounds(answer) = views.answer(&wire::Query::Rounds { run }) else {
        panic!("Rounds answers with rounds");
    };
    *answer
}

/// A conversation has a first line and a last line, and neither is a
/// turn: the task the person gave, and the word the run froze with. The
/// first carries the frozen facts a page heads the session with - its
/// policy, its effort and the names its session froze - read from the
/// line and from the store, never from today's settings.
#[test]
fn the_opening_carries_the_effort_and_the_names_the_run_froze() {
    let dir = tempfile::tempdir().unwrap();
    let answer = rounds_of(
        dir.path(),
        RunId::from_bytes([8u8; 16]),
        // A whole locator, because `run_started` states one and
        // `kernel::event::record::RunStarted` parses it.
        serde_json::json!({
            "task": "plan the week",
            "goal": "a roadmap",
            "job": "file:hall/mayor@0123456789abcdef0123456789abcdef01234567",
            "dispatched_by": "person",
            "policy": { "mode": "work", "write": "create", "admit": "tested",
                        "landing": "experiment" },
            "naming": B3Hash::digest(NAMED_CAT).to_string(),
            "effort": "high",
        }),
        &[NAMED_CAT],
    );
    assert_eq!(
        answer.opening,
        Some(wire::Opening {
            task: "plan the week".to_owned(),
            goal: "a roadmap".to_owned(),
            at: TimeMs::new(10),
            dispatched_by: Some(kernel::event::Who::Person),
            policy: Some(kernel::RunPolicy {
                mode: kernel::Mode::Work,
                write: kernel::WriteLimit::Create,
                admit: kernel::AdmissionRequirement::Tested,
                landing: kernel::LandingPolicy::Experiment,
            }),
            effort: Some(kernel::Effort::High),
            names: Some(wire::FrozenNames {
                mayor: Some("Cat".to_owned()),
            }),
        }),
        "the opening carries what run_started records, and the names its version holds"
    );
    assert_eq!(answer.worktree.as_deref(), Some("hall-mayor"));
    let closing = answer.closing.expect("the window held run_frozen");
    assert_eq!(closing.completion, "done");
    assert_eq!(closing.at, TimeMs::new(12));
}

/// A version the store does not hold names nobody: the page falls back to
/// the address rather than to whatever the names are today.
#[test]
fn a_naming_the_store_lost_answers_no_names() {
    let dir = tempfile::tempdir().unwrap();
    let answer = rounds_of(
        dir.path(),
        RunId::from_bytes([9u8; 16]),
        serde_json::json!({ "task": "t", "naming": B3Hash::digest(NAMED_CAT).to_string() }),
        &[],
    );
    let opening = answer.opening.expect("the window held run_started");
    assert_eq!((opening.names, opening.effort), (None, None));
}
