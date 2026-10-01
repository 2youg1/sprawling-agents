// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A start from a snapshot folds only the tail into the views a start
//! from genesis builds; a snapshot that fails verification is refused.

use std::path::{Path, PathBuf};

use kernel::{Address, RunId, Seq};
use storage::{StoredSnapshot, WholeFold};

use super::*;
use crate::views::snapshot::start::{
    FoldStart, cut, proof_dir, snapshot_dir, start, start_audited,
};
use crate::worker::RunWorker;

fn raise(worker: &mut RunWorker, names: std::ops::Range<u8>) {
    for n in names {
        worker
            .handle(wire::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: wire::TemplateName::parse("minimal").unwrap(),
                idem: kernel::IdemKey::derive(&RunId::CITY, Seq::FIRST, &[n]),
            })
            .unwrap();
    }
}

fn snapshots(city: &Path) -> PathBuf {
    snapshot_dir::<Views>(city)
}

/// A city with three buildings, a snapshot cut after them, then `more`
/// buildings the snapshot has not seen.
fn cut_then_raise(city: &Path, more: std::ops::Range<u8>) -> (PathBuf, RunWorker) {
    let ledger = crate::worker::fixture::init_city(city).unwrap().ledger_dir;
    let mut worker = RunWorker::new(
        city,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    raise(&mut worker, 0..3);
    cut(&ledger, &start::<Views>(&ledger).unwrap()).unwrap();
    let cut = storage::read_snapshot(&snapshots(city)).unwrap();
    assert!(matches!(cut, StoredSnapshot::Present(_)), "{cut:?}");
    raise(&mut worker, more);
    (ledger, worker)
}

fn from_genesis(city: &Path, ledger: &Path) -> Vec<u8> {
    std::fs::remove_dir_all(snapshots(city)).unwrap();
    let whole = start::<Views>(ledger).unwrap();
    assert_eq!(whole.from, FoldStart::Whole(WholeFold::NoSnapshot));
    whole.folded.encode().unwrap()
}

#[test]
fn a_start_after_a_cut_folds_only_the_tail_into_the_same_views() {
    let dir = tempfile::tempdir().unwrap();
    let (ledger, _worker) = cut_then_raise(dir.path(), 3..6);
    let lines = storage::read_raw_lines_at(&ledger).unwrap().len();
    let StoredSnapshot::Present(cut) = storage::read_snapshot(&snapshots(dir.path())).unwrap()
    else {
        panic!("the snapshot is still there");
    };
    let tail = lines - usize::try_from(cut.seq().value()).unwrap() - 1;

    let resumed = start::<Views>(&ledger).unwrap();

    assert_eq!(resumed.from, FoldStart::Resumed { tail });
    assert_eq!(
        resumed.folded.encode().unwrap(),
        from_genesis(dir.path(), &ledger)
    );
}

#[test]
fn a_tampered_snapshot_is_refused_and_the_whole_history_is_folded() {
    let dir = tempfile::tempdir().unwrap();
    let (ledger, _worker) = cut_then_raise(dir.path(), 3..5);
    let file = std::fs::read_dir(snapshots(dir.path()))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut bytes = std::fs::read(&file).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    std::fs::write(&file, bytes).unwrap();

    let refused = start::<Views>(&ledger).unwrap();

    assert!(
        matches!(refused.from, FoldStart::Whole(WholeFold::Damaged(_))),
        "{:?}",
        refused.from
    );
    assert_eq!(
        refused.folded.encode().unwrap(),
        from_genesis(dir.path(), &ledger)
    );
}

#[test]
fn views_a_snapshot_cannot_decode_are_folded_from_genesis() {
    let dir = tempfile::tempdir().unwrap();
    let (ledger, _worker) = cut_then_raise(dir.path(), 3..4);
    let lines = storage::read_raw_lines_at(&ledger).unwrap();
    let seq = Seq::new(u64::try_from(lines.len()).unwrap() - 1);
    let snapshot = storage::ChainSnapshot::cut(
        crate::views::snapshot::views_fold_version(),
        seq,
        lines.last().unwrap(),
        b"not the views".to_vec(),
    );
    storage::write_snapshot(&snapshots(dir.path()), &snapshot).unwrap();

    let refused = start::<Views>(&ledger).unwrap();

    assert!(
        matches!(refused.from, FoldStart::Whole(WholeFold::Damaged(_))),
        "{:?}",
        refused.from
    );
    assert_eq!(
        refused.folded.encode().unwrap(),
        from_genesis(dir.path(), &ledger)
    );
}

#[test]
fn a_one_shot_read_refuses_a_line_edited_before_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let (ledger, worker) = cut_then_raise(dir.path(), 3..4);
    drop(worker);
    let first = storage::ledger_segments_at(&ledger).unwrap().remove(0);
    let mut bytes = std::fs::read(&first).unwrap();
    let at = bytes.windows(4).position(|held| held == b"\"t\":").unwrap() + 4;
    bytes[at] = if bytes[at] == b'1' { b'2' } else { b'1' };
    std::fs::write(&first, bytes).unwrap();
    let storage::ChainAudit::Broken(reason) = storage::audit_chain(&ledger).unwrap() else {
        panic!("the edited ledger still audits whole");
    };

    let read = Views::rebuild(&ledger).map(|_| ());

    assert_eq!(read, Err(reason));
}

#[test]
fn a_snapshot_that_cannot_be_cut_is_reported_and_the_views_still_serve() {
    let dir = tempfile::tempdir().unwrap();
    let (ledger, worker) = cut_then_raise(dir.path(), 3..4);
    drop(worker);
    // A directory where the cut stages its file: removing it as a file
    // fails on every platform, and the read path never looks there.
    std::fs::create_dir_all(snapshots(dir.path()).join("chain.snap.staged/held")).unwrap();
    let said = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let heard = std::sync::Arc::clone(&said);
    let mut log = Diagnostics::new(
        Level::Effect,
        Box::new(move |entry: runtime::diagnostics::Entry<'_>| {
            heard
                .lock()
                .unwrap()
                .push((entry.level, entry.message.to_owned()));
        }),
    );

    let served = start_served_views(
        &ledger,
        crate::Clock::now(&crate::worker::fixture::WallClock).unwrap(),
        &mut log,
        &mut crate::worker::opening_cost::OpeningCost::begin(crate::worker::fixture::monotonic),
    )
    .map(|_| ());

    assert_eq!(served, Ok(()));
    let said = said.lock().unwrap();
    assert!(
        said.iter()
            .any(|(level, message)| *level == Level::Refuse && message.contains("snapshot")),
        "{said:?}"
    );
}

/// A served city whose last opening cut both snapshots opens again from
/// them: before the first byte it checks and folds only the lines written
/// since, not the history before them (sprawling-SPEC.md 8-122).
#[test]
fn a_served_city_reopens_from_its_snapshots_and_folds_only_what_grew() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = crate::worker::fixture::init_city(dir.path())
        .unwrap()
        .ledger_dir;
    let open = |ledger: &Path| {
        let mut cost =
            crate::worker::opening_cost::OpeningCost::begin(crate::worker::fixture::monotonic);
        let served = start_served_views(
            ledger,
            crate::Clock::now(&crate::worker::fixture::WallClock).unwrap(),
            &mut Diagnostics::off(),
            &mut cost,
        )
        .unwrap();
        drop(served);
        cost.line()
    };
    open(&ledger);
    let opened_at = storage::read_raw_lines_at(&ledger).unwrap().len();
    let mut worker = RunWorker::new(
        dir.path(),
        Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    raise(&mut worker, 0..2);
    drop(worker);
    let grown = storage::read_raw_lines_at(&ledger).unwrap().len() - opened_at;

    let line = open(&ledger);

    let fold = line
        .split(", ")
        .find(|part| part.contains(" lines "))
        .and_then(|part| part.rsplitn(3, ' ').nth(2).map(str::to_owned));
    assert_eq!(
        fold,
        Some(format!("fold {grown} lines from the snapshots")),
        "{line}"
    );
}

/// A one-shot rebuild checks each line once: from genesis the fold's own
/// check is the proof, and from a snapshot with the proof records full it
/// checks only what was written since - twice, once proving and once
/// folding - at any length of history (accounting-SPEC.md 8-19).
#[test]
fn a_rebuild_checks_each_line_once_whatever_the_length_of_history() {
    let hands = crate::worker::fixture::hands;
    let checked = |buildings: u8| {
        let dir = tempfile::tempdir().unwrap();
        let city = dir.path();
        let ledger = crate::worker::fixture::init_city(city).unwrap().ledger_dir;
        let mut worker = RunWorker::new(city, Diagnostics::off(), hands()).unwrap();
        raise(&mut worker, 0..buildings);
        drop(worker);
        let count = |ledger: &Path| {
            u64::try_from(storage::read_raw_lines_at(ledger).unwrap().len()).unwrap()
        };
        let lines = count(&ledger);
        let whole = start_audited::<Views>(&ledger).unwrap();
        cut(&ledger, &whole.started).unwrap();
        let (held, _) = storage::JsonlLedger::open(&ledger, kernel::TimeMs::new(0)).unwrap();
        storage::prove_chain(&ledger, &held.proof_records(&proof_dir(city))).unwrap();
        drop(held);
        let mut worker = RunWorker::new(city, Diagnostics::off(), hands()).unwrap();
        raise(&mut worker, 100..102);
        drop(worker);
        let since = count(&ledger) - lines;
        let resumed = start_audited::<Views>(&ledger).unwrap();
        assert!(matches!(resumed.started.from, FoldStart::Resumed { .. }));
        (
            (whole.lines_checked, resumed.lines_checked),
            (lines, 2 * since),
        )
    };
    for buildings in [3, 6] {
        let (counted, expected) = checked(buildings);
        assert_eq!(counted, expected, "{buildings} buildings");
    }
}
