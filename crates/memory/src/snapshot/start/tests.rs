// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::Seq;

use super::*;
use crate::jsonl::segment_file_name;
use crate::snapshot::tests::chain;
use crate::snapshot::write_snapshot;

/// Lay `lines` out as a ledger of segments that each start at one of `firsts`.
fn lay_out(dir: &Path, lines: &[Vec<u8>], firsts: &[u64]) {
    std::fs::create_dir_all(dir).unwrap();
    for (i, first) in firsts.iter().enumerate() {
        let end = firsts.get(i + 1).map_or(lines.len(), |next| *next as usize);
        let body: Vec<u8> = lines[*first as usize..end]
            .iter()
            .flat_map(|line| line.iter().copied().chain(*b"\n"))
            .collect();
        std::fs::write(dir.join(segment_file_name(Seq::new(*first))), body).unwrap();
    }
}

#[test]
fn a_fitting_snapshot_resumes_at_its_tail_without_reading_earlier_segments() {
    let tmp = tempfile::tempdir().unwrap();
    let (ledger, snapshots) = (tmp.path().join("ledger"), tmp.path().join("snapshot"));
    let lines = chain(8, 0);
    lay_out(&ledger, &lines, &[0, 4]);
    let snapshot = ChainSnapshot::cut(3, Seq::new(5), &lines[5], b"views".to_vec());
    write_snapshot(&snapshots, &snapshot).unwrap();
    // A segment before the snapshot's is never read: damage there cannot
    // change where this start begins.
    std::fs::write(ledger.join(segment_file_name(Seq::new(0))), b"not a line\n").unwrap();

    let start = start_from_snapshot(&ledger, &snapshots, 3).unwrap();

    assert_eq!(
        start,
        SnapshotStart::Resume {
            snapshot,
            tail: lines[6..].to_vec(),
        }
    );
}

#[test]
fn a_snapshot_that_cannot_resume_folds_the_whole_ledger_and_says_why() {
    let tmp = tempfile::tempdir().unwrap();
    let (ledger, snapshots) = (tmp.path().join("ledger"), tmp.path().join("snapshot"));
    let (ours, theirs) = (chain(8, 0), chain(8, 1_000));
    lay_out(&ledger, &ours, &[0, 4]);
    let whole = |because| SnapshotStart::Whole {
        lines: ours.clone(),
        because,
    };
    let start_after = |snapshot: &ChainSnapshot, fold_version| {
        write_snapshot(&snapshots, snapshot).unwrap();
        start_from_snapshot(&ledger, &snapshots, fold_version).unwrap()
    };

    assert_eq!(
        start_from_snapshot(&ledger, &snapshots, 3).unwrap(),
        whole(WholeFold::NoSnapshot)
    );
    let fits = ChainSnapshot::cut(3, Seq::new(2), &ours[2], Vec::new());
    assert_eq!(
        start_after(&fits, 4),
        whole(WholeFold::OtherFoldVersion { found: 3 })
    );
    let stale = ChainSnapshot::cut(3, Seq::new(6), &theirs[6], Vec::new());
    assert_eq!(start_after(&stale, 3), whole(WholeFold::Stale));
    let beyond = ChainSnapshot::cut(3, Seq::new(9), &theirs[7], Vec::new());
    assert_eq!(start_after(&beyond, 3), whole(WholeFold::Missing));
    std::fs::write(snapshots.join("chain.snap"), b"SPRSNAP1").unwrap();
    assert_eq!(
        start_from_snapshot(&ledger, &snapshots, 3).unwrap(),
        whole(WholeFold::Damaged(
            "the file ends inside its header".to_owned()
        ))
    );
}
