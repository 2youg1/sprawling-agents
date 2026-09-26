// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A worker's standing started from its snapshot is the standing a
//! start from genesis builds, byte for byte.

use kernel::{Address, RunId, Seq};
use memory::{StoredSnapshot, WholeFold};

use super::*;
use crate::assembly::{RunWorker, init_city};
use crate::views::snapshot::start::{FoldStart, snapshot_dir, start};

/// Buildings named `lab<n>`, and one repeat of `lab0` under a new key,
/// so the refusals `Entrance` keeps are in the folds too.
fn raise(worker: &mut RunWorker, names: std::ops::Range<u8>) {
    for n in names {
        let create = |name: &str, key: u8| channels::Command::CreateBuilding {
            addr: Address::parse(name).unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, Seq::FIRST, &[n, key]),
        };
        worker.handle(create(&format!("lab{n}"), 0)).unwrap();
        let repeat = worker.handle(create("lab0", 1));
        assert!(n == 0 || repeat.is_err(), "{repeat:?}");
    }
}

#[test]
fn a_standing_after_a_cut_folds_only_the_tail_into_the_same_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = init_city(dir.path()).unwrap().ledger_dir;
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    raise(&mut worker, 0..3);
    let snapshots = snapshot_dir::<StandingFolds>(dir.path());

    Standing::fold(&ledger).unwrap();

    let cut = memory::read_snapshot(&snapshots).unwrap();
    assert!(matches!(cut, StoredSnapshot::Present(_)), "{cut:?}");
    let StoredSnapshot::Present(cut) = cut else {
        return;
    };
    raise(&mut worker, 3..5);
    let lines = memory::read_raw_lines_at(&ledger).unwrap().len();
    let tail = lines - usize::try_from(cut.seq().value()).unwrap() - 1;
    let resumed = start::<StandingFolds>(&ledger).unwrap();
    std::fs::remove_dir_all(&snapshots).unwrap();
    let whole = start::<StandingFolds>(&ledger).unwrap();
    assert_eq!(
        (resumed.from, whole.from),
        (
            FoldStart::Resumed { tail },
            FoldStart::Whole(WholeFold::NoSnapshot)
        )
    );
    assert_eq!(
        resumed.folded.encode().unwrap(),
        whole.folded.encode().unwrap()
    );
}

/// Every line before the snapshot's sealed into a segment of its own,
/// then the first line rewritten into another line that is still
/// canonical and still continues genesis: only the second line's `prev`
/// breaks, in a segment the open's tail scan never reads, and the
/// snapshot's fit checks only the line at its seq.
#[test]
fn a_worker_refuses_a_line_rewritten_before_the_snapshot_in_a_sealed_segment() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = init_city(dir.path()).unwrap().ledger_dir;
    let open = || {
        RunWorker::new(
            dir.path(),
            gateway::Custodian::in_memory(),
            runtime::diagnostics::Diagnostics::off(),
        )
    };
    raise(&mut open().unwrap(), 0..3);
    Standing::fold(&ledger).unwrap();
    let StoredSnapshot::Present(cut) =
        memory::read_snapshot(&snapshot_dir::<StandingFolds>(dir.path())).unwrap()
    else {
        panic!("the fold cut no snapshot");
    };
    let segment = memory::ledger_segments_at(&ledger).unwrap().remove(0);
    let bytes = std::fs::read(&segment).unwrap();
    let lines: Vec<&[u8]> = bytes.split_inclusive(|held| *held == b'\n').collect();
    let (sealed, open_segment) = lines.split_at(usize::try_from(cut.seq().value()).unwrap());
    let mut sealed = sealed.concat();
    let at = sealed
        .windows(4)
        .position(|held| held == b"\"t\":")
        .unwrap()
        + 4;
    assert!(at < lines[0].len(), "the first line carries no timestamp");
    sealed[at] = if sealed[at] == b'1' { b'2' } else { b'1' };
    std::fs::write(&segment, sealed).unwrap();
    std::fs::write(
        ledger.join(format!("ledger-{:020}.jsonl", cut.seq().value())),
        open_segment.concat(),
    )
    .unwrap();

    let refused = open().map(|_| ());

    assert_eq!(refused, Err(memory::LineFault::ChainBreak.into_ax(2)));
}
