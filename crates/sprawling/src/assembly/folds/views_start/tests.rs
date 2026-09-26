// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A start from a snapshot folds only the tail into the views a start
//! from genesis builds; a snapshot that fails verification is refused.

use std::path::{Path, PathBuf};

use kernel::{Address, RunId, Seq};
use memory::{StoredSnapshot, WholeFold};

use super::super::snapshot_start::{FoldStart, snapshot_dir};
use super::*;
use crate::assembly::{RunWorker, init_city};

fn raise(worker: &mut RunWorker, names: std::ops::Range<u8>) {
    for n in names {
        worker
            .handle(channels::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: channels::TemplateName::parse("minimal").unwrap(),
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
    let ledger = init_city(city).unwrap().ledger_dir;
    let mut worker = RunWorker::new(
        city,
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    raise(&mut worker, 0..3);
    cut(&ledger, &start_views(&ledger).unwrap()).unwrap();
    let cut = memory::read_snapshot(&snapshots(city)).unwrap();
    assert!(matches!(cut, StoredSnapshot::Present(_)), "{cut:?}");
    raise(&mut worker, more);
    (ledger, worker)
}

fn from_genesis(city: &Path, ledger: &Path) -> Vec<u8> {
    std::fs::remove_dir_all(snapshots(city)).unwrap();
    let whole = start_views(ledger).unwrap();
    assert_eq!(whole.from, FoldStart::Whole(WholeFold::NoSnapshot));
    whole.folded.encode().unwrap()
}

#[test]
fn a_start_after_a_cut_folds_only_the_tail_into_the_same_views() {
    let dir = tempfile::tempdir().unwrap();
    let (ledger, _worker) = cut_then_raise(dir.path(), 3..6);
    let lines = memory::read_raw_lines_at(&ledger).unwrap().len();
    let StoredSnapshot::Present(cut) = memory::read_snapshot(&snapshots(dir.path())).unwrap()
    else {
        panic!("the snapshot is still there");
    };
    let tail = lines - usize::try_from(cut.seq().value()).unwrap() - 1;

    let resumed = start_views(&ledger).unwrap();

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

    let refused = start_views(&ledger).unwrap();

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
    let lines = memory::read_raw_lines_at(&ledger).unwrap();
    let seq = Seq::new(u64::try_from(lines.len()).unwrap() - 1);
    let snapshot = memory::ChainSnapshot::cut(
        crate::views::views_fold_version(),
        seq,
        lines.last().unwrap(),
        b"not the views".to_vec(),
    );
    memory::write_snapshot(&snapshots(dir.path()), &snapshot).unwrap();

    let refused = start_views(&ledger).unwrap();

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
    let first = memory::ledger_segments_at(&ledger).unwrap().remove(0);
    let mut bytes = std::fs::read(&first).unwrap();
    let at = bytes.windows(4).position(|held| held == b"\"t\":").unwrap() + 4;
    bytes[at] = if bytes[at] == b'1' { b'2' } else { b'1' };
    std::fs::write(&first, bytes).unwrap();
    let memory::ChainAudit::Broken(reason) = memory::audit_chain(&ledger).unwrap() else {
        panic!("the edited ledger still audits whole");
    };

    let read = crate::assembly::rebuild_views(&ledger).map(|_| ());

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

    let served = start_served_views(&ledger, &mut log).map(|_| ());

    assert_eq!(served, Ok(()));
    let said = said.lock().unwrap();
    assert!(
        said.iter()
            .any(|(level, message)| *level == Level::Refuse && message.contains("snapshot")),
        "{said:?}"
    );
}
