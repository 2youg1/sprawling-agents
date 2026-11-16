// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code,
    reason = "test code; readings are read through their Debug form in the report"
)]

use std::sync::Arc;

use kernel::EventKind;

use super::ledger::{
    city, dispatch, frozen, frozen_seq, raise, serve,
    started,
};
use super::roles::{
    CHILDREN, Child, Counts, ROUNDS, Roles, Send, Shared, Siblings,
    play,
};

#[test]
fn tp3_three_delegated_children_start_at_the_call_and_run_side_by_side() {
    let dir = tempfile::tempdir().unwrap();
    let met = Arc::new(Siblings::default());
    let factory = Roles::new(Arc::clone(&met), Shared::default(), Send::Async);
    let (mut worker, ledger) = city(dir.path(), factory);
    raise(&mut worker, "lab");
    let lines = serve(
        worker,
        &ledger,
        vec![dispatch("lab/lead", "three rooms at work")],
        |lines| frozen(lines) == 4,
    );

    let lead = started(&lines, "lab/lead").remove(0);
    let delegated: Vec<u64> = lines
        .iter()
        .filter(|(record, _)| record.kind() == EventKind::ToolCalled && record.run() == lead.run)
        .map(|(record, _)| record.seq().value())
        .collect();
    let lead_frozen = frozen_seq(&lines, lead.run);
    let children: Vec<Child> = CHILDREN
        .iter()
        .map(|room| {
            let start = started(&lines, room).remove(0);
            Child {
                started: start.seq,
                frozen: frozen_seq(&lines, start.run),
                parent: start.parent == Some(lead.run),
            }
        })
        .collect();
    let report = format!(
        "delegate calls at seq {delegated:?}, lead frozen at {lead_frozen}, children {children:?}, \
         lead started at t={} and children at t={:?}",
        lead.t,
        CHILDREN
            .iter()
            .map(|room| started(&lines, room).remove(0).t)
            .collect::<Vec<_>>()
    );
    println!("{report}");
    assert_eq!(
        (
            children.iter().all(|child| child.parent),
            children
                .iter()
                .zip(&delegated)
                .all(|(child, call)| child.started > *call && child.frozen != u64::MAX),
            met.all_met(),
        ),
        (true, true, true),
        "each child starts at its call, under the lead, and the three run side by side: {report}"
    );
}

#[test]
fn tp3_the_second_game_under_the_asynchronous_send() {
    let reading = play(Send::Async);
    println!("asynchronous: {reading:?}");
    let serial = 1 + 2 * ROUNDS;
    assert!(
        reading.runs == reading.reads_at_start
            && (serial - 1..=serial).contains(&reading.runs)
            && reading.model_calls <= 7 * 2 + 2 * ROUNDS + 1,
        "{reading:?}"
    );
}

#[test]
fn tp3_the_second_game_under_the_synchronous_send() {
    let reading = play(Send::Sync);
    println!("synchronous: {reading:?}");
    assert_eq!(
        reading.counts(),
        Counts {
            runs: 1 + ROUNDS,
            reads_at_start: 1 + ROUNDS,
            model_calls: 1 + ROUNDS + 1 + ROUNDS * 3,
        },
        "{reading:?}"
    );
}
