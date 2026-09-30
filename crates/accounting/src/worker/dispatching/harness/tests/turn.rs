// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whole turns of a played harness, from the person's door to the
//! landing: done and offered, and cut at the building's ceiling.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kernel::event::record::RunFrozen;
use kernel::{AxError, Completion, EventKind, EventRecord, Evidence, Payload, TimeMs};
use serde_json::json;

use super::{Heard, Step, city_with, dispatch, opening, played, records, said, worker};

/// The whole turn: the harness works in the room's own tree, reports,
/// and ends its turn with an answer. The run's lines are the order
/// Session.lean proves, and it freezes done citing its answer.
#[test]
fn a_harness_that_ends_its_turn_is_frozen_done_on_its_answer() {
    let (dir, _, ledger) = city_with("pi");
    let heard = Arc::new(Mutex::new(Heard::default()));
    let mut script = opening();
    script.extend([
        Step::Write("lab/room1/notes.md", "# Notes\n\nThey read well now.\n"),
        said("the notes read well"),
        Step::Answer(json!({ "stopReason": "end_turn" })),
        Step::Drain,
    ]);
    let mut worker = worker(dir.path(), played(script, Arc::clone(&heard)));

    assert_eq!(worker.handle(dispatch("lab/room1", None)), Ok(()));

    let all = records(&ledger);
    let started = all
        .iter()
        .find(|record| record.kind() == EventKind::RunStarted)
        .unwrap()
        .run();
    let run: Vec<&EventRecord> = all
        .iter()
        .filter(|record| record.run() == started)
        .collect();
    assert_eq!(
        run.iter().map(|record| record.kind()).collect::<Vec<_>>(),
        vec![
            EventKind::WorktreeOpened,
            EventKind::RunStarted,
            EventKind::HarnessReported,
            EventKind::CheckpointCommitted,
            EventKind::HarnessAnswered,
            EventKind::HandoffWritten,
            EventKind::RunFrozen,
            EventKind::PrOpened,
        ]
    );
    let answer = run
        .iter()
        .find(|record| record.kind() == EventKind::HarnessAnswered)
        .unwrap()
        .to_ref();
    let frozen = run
        .iter()
        .find(|record| record.kind() == EventKind::RunFrozen)
        .unwrap();
    assert_eq!(
        frozen.data(),
        &Payload::of(&RunFrozen::of(&Completion::Done(
            Evidence::new(vec![answer]).unwrap()
        )))
        .unwrap()
    );

    let heard = heard.lock().unwrap();
    let tree = heard.cwd.clone().unwrap();
    assert_ne!(tree, dir.path(), "the harness ran in the room's own tree");
    assert!(tree.join("lab/room1/notes.md").is_file());
    assert!(
        !dir.path().join("lab/room1/notes.md").exists(),
        "what the harness wrote waits in its tree until a merge"
    );
    let prompted = heard
        .messages
        .iter()
        .find(|message| message["method"] == "session/prompt")
        .unwrap();
    assert!(
        prompted["params"]["prompt"][0]["text"]
            .as_str()
            .unwrap()
            .contains("fix the notes"),
        "the harness is prompted with the room's brief"
    );
}

/// The kinds of the one run in the history, and that run's records.
fn the_run(ledger: &Path) -> Vec<EventRecord> {
    let all = records(ledger);
    let started = all
        .iter()
        .find(|record| record.kind() == EventKind::RunStarted)
        .unwrap()
        .run();
    all.into_iter()
        .filter(|record| record.run() == started)
        .collect()
}

/// A run frozen done offers its tree: the city opens the request the
/// harness cannot open itself, on the branch of the tree it worked in,
/// and the existing review decides what reaches the building.
#[test]
fn a_harness_run_frozen_done_offers_its_tree_for_review() {
    let (dir, _, ledger) = city_with("pi");
    let mut script = opening();
    script.extend([
        Step::Write("lab/room1/notes.md", "# Notes\n"),
        said("written"),
        Step::Answer(json!({ "stopReason": "end_turn" })),
        Step::Drain,
    ]);
    let mut worker = worker(
        dir.path(),
        played(script, Arc::new(Mutex::new(Heard::default()))),
    );

    assert_eq!(worker.handle(dispatch("lab/room1", None)), Ok(()));

    let run = the_run(&ledger);
    let lent = run
        .iter()
        .find(|record| record.kind() == EventKind::WorktreeOpened)
        .unwrap()
        .data()
        .as_map()
        .get("name")
        .cloned();
    let offered = run
        .iter()
        .find(|record| record.kind() == EventKind::PrOpened)
        .map(|record| record.data().as_map().get("branch").cloned());
    assert_eq!(
        offered,
        Some(lent),
        "the request names the tree the harness worked in"
    );
}

/// A clock that moves thirty seconds every time it is read.
struct Hurried(AtomicU64);

impl crate::Clock for Hurried {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(self.0.fetch_add(30_000, Ordering::Relaxed)))
    }
}

/// A harness that neither speaks nor ends is cut at the building's
/// ceiling: the cancel goes out, the harness answers cancelled, and the
/// run freezes as limit, not as a run somebody cancelled.
#[test]
fn a_harness_that_says_nothing_is_cut_at_the_ceiling_and_frozen_limit() {
    let (dir, _, ledger) = city_with("pi");
    crate::worker::fixture::lay_rules(
        dir.path(),
        "lab",
        &crate::worker::fixture::ordinary_rules("harness_minutes = 1\n"),
    );
    let mut script = opening();
    script.extend([
        Step::Heed {
            within: Duration::from_secs(3),
            heard: json!({ "stopReason": "cancelled" }),
            otherwise: json!({ "stopReason": "end_turn" }),
        },
        Step::Drain,
    ]);
    let since = crate::Clock::now(&crate::worker::fixture::WallClock).unwrap();
    let mut worker = worker(
        dir.path(),
        played(script, Arc::new(Mutex::new(Heard::default()))),
    )
    .with_clock(Arc::new(Hurried(AtomicU64::new(since.value()))));

    assert_eq!(worker.handle(dispatch("lab/room1", None)), Ok(()));

    let run = the_run(&ledger);
    assert_eq!(
        run.iter().map(EventRecord::kind).collect::<Vec<_>>(),
        vec![
            EventKind::WorktreeOpened,
            EventKind::RunStarted,
            EventKind::CancelReceived,
            EventKind::CheckpointCommitted,
            EventKind::HarnessAnswered,
            EventKind::HandoffWritten,
            EventKind::RunFrozen,
        ]
    );
    assert_eq!(
        run.last().unwrap().data(),
        &Payload::of(&RunFrozen::of(&Completion::Limit)).unwrap()
    );
}
