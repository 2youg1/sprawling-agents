// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod instruments;

use std::sync::{Arc, mpsc};
use std::time::Duration;

use kernel::EventRecord;

use super::{Broadcast, CUT_WAITS_ABOVE, Cadence, Copies, Folded, spawn_folding};
use crate::assembly::init_city;
use crate::serving::standing::monotonic_now;
use accounting::person::CorePriority;
use accounting::views::snapshot::start::{FoldStart, start};
use accounting::views::{Published, Views, answer_outside_the_lock};

/// A reader in the middle of a query holds up neither the writer nor the
/// fold: the observer returns, and the record is folded and broadcast,
/// while the reader still holds the views it is answering from.
#[test]
fn a_reader_holding_the_views_holds_up_neither_the_writer_nor_the_fold() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let genesis = EventRecord::parse_line(verified.raw_lines().first().unwrap()).unwrap();
    let unfolded = Views::new(dir.path());
    let spare = unfolded.twin().unwrap();
    let views = Arc::new(Published::new(unfolded));
    let (to_clients, mut heard) = tokio::sync::broadcast::channel(8);
    let head = Arc::new(wire::LedgerHead::default());
    let broadcast = Broadcast {
        to_clients,
        head: Arc::clone(&head),
    };
    let copies = Copies {
        published: Arc::clone(&views),
        spare,
    };
    let mut folding =
        spawn_folding(copies, broadcast, CorePriority::Raised, monotonic_now).unwrap();

    let reader = views.snapshot();
    let (written, returned) = mpsc::channel();
    let sent = genesis.clone();
    let writer = std::thread::spawn(move || {
        (folding.observer)(&sent);
        written.send(()).unwrap();
        folding
    });
    let waited = returned.recv_timeout(Duration::from_secs(2));
    let heard_while_held = heard_within(&mut heard, Duration::from_secs(2));
    drop(reader);
    assert_eq!(
        (waited, heard_while_held),
        (Ok(()), Some(genesis.clone())),
        "the writer or the fold waited for the reader"
    );

    let folding = writer.join().unwrap();
    drop((
        folding.observer,
        folding.machine,
        folding.lend,
        folding.keep_slices,
    ));
    folding.thread.join().unwrap();
    assert_eq!(head.read(), Some(genesis.seq()));
    let mut folded_here = Views::new(dir.path());
    folded_here.apply(&genesis).unwrap();
    let city = wire::Query::CityView;
    let (as_of, answered) = answer_outside_the_lock(&views, &city);
    assert_eq!(
        (as_of, answered.unwrap()),
        (
            genesis.seq().next().unwrap(),
            folded_here.prepare(&city).finish()
        )
    );
}

/// A served city cuts its views snapshot on the fold thread, at the last
/// record it folded, so a later read resumes there and folds no tail.
#[test]
fn the_fold_thread_cuts_a_snapshot_a_later_read_resumes_from() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let records: Vec<EventRecord> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .collect();
    let unfolded = Views::new(dir.path());
    let spare = unfolded.twin().unwrap();
    let copies = Copies {
        published: Arc::new(Published::new(unfolded)),
        spare,
    };
    let broadcast = Broadcast {
        to_clients: tokio::sync::broadcast::channel(8).0,
        head: Arc::new(wire::LedgerHead::default()),
    };
    let mut folding =
        spawn_folding(copies, broadcast, CorePriority::Normal, monotonic_now).unwrap();
    for record in &records {
        (folding.observer)(record);
    }
    drop((
        folding.observer,
        folding.machine,
        folding.lend,
        folding.keep_slices,
    ));
    folding.thread.join().unwrap();

    let resumed = start::<Views>(&report.ledger_dir).unwrap();
    assert_eq!(resumed.from, FoldStart::Resumed { tail: 0 });
}

/// The first record broadcast within `patience`, polled rather than
/// awaited so a fold that never broadcasts fails the test instead of
/// hanging it.
#[allow(
    clippy::disallowed_methods,
    clippy::arithmetic_side_effects,
    reason = "test code: the patience is read off the wall clock"
)]
fn heard_within(
    heard: &mut tokio::sync::broadcast::Receiver<wire::Committed>,
    patience: Duration,
) -> Option<EventRecord> {
    let until = std::time::Instant::now() + patience;
    loop {
        if let Ok(committed) = heard.try_recv() {
            return Some(committed.record().clone());
        }
        if std::time::Instant::now() >= until {
            return None;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// A cut that falls due waits while the fold is further behind the writer
/// than the bound, and is handed out again once it has caught up
/// (sprawling-SPEC.md 8-123).
#[test]
fn a_cut_waits_while_the_fold_is_behind() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let genesis = EventRecord::parse_line(
        storage::read_raw_lines_at(&report.ledger_dir)
            .unwrap()
            .first()
            .unwrap(),
    )
    .unwrap();
    let mut cadence = Cadence::default();
    cadence.folded(Duration::from_millis(1), Some(&genesis), Folded::Clean);

    assert_eq!(
        (
            cadence.due(CUT_WAITS_ABOVE + 1).map(|record| record.seq()),
            cadence.due(CUT_WAITS_ABOVE).map(|record| record.seq()),
        ),
        (None, Some(genesis.seq()))
    );
}
