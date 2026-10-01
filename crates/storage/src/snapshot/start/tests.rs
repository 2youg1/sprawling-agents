// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::{EventDraft, EventKind, Payload, RunId, Seq, TimeMs};

use super::*;
use crate::jsonl::{JsonlLedger, ledger_segments_at, segment_file_name};
use crate::snapshot::tests::chain;
use crate::snapshot::write_snapshot;
use crate::vfs::Vfs;

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

/// The cut is found from its segment's end, so a line damaged before it
/// in the same segment changes where a start begins no more than one in
/// an earlier segment does; the proof checks both (`crates/storage/spec/Snapshot/Start.lean` §8-28).
#[test]
fn damage_before_the_cut_in_its_own_segment_does_not_move_the_start() {
    let tmp = tempfile::tempdir().unwrap();
    let (ledger, snapshots) = (tmp.path().join("ledger"), tmp.path().join("snapshot"));
    let lines = chain(8, 0);
    lay_out(&ledger, &lines, &[0]);
    let snapshot = ChainSnapshot::cut(3, Seq::new(5), &lines[5], b"views".to_vec());
    write_snapshot(&snapshots, &snapshot).unwrap();
    let segment = ledger.join(segment_file_name(Seq::new(0)));
    let damaged = std::fs::read_to_string(&segment)
        .unwrap()
        .replacen('\n', "\nnot a line\n", 1);
    std::fs::write(&segment, damaged).unwrap();

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
    let whole = SnapshotStart::Whole;
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

/// The second adapter of the `Vfs` seam this test needs: the real disk,
/// with every byte read counted against the file it came from.
#[derive(Clone)]
struct CountingFs {
    disk: Arc<Mutex<RealFs>>,
    read: Arc<Mutex<BTreeMap<String, u64>>>,
}

impl CountingFs {
    fn new() -> CountingFs {
        CountingFs {
            disk: Arc::new(Mutex::new(RealFs::new())),
            read: Arc::default(),
        }
    }

    fn count(&self, path: &Path, bytes: &io::Result<Vec<u8>>) {
        if let Ok(bytes) = bytes {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            *self.read.lock().unwrap().entry(name).or_default() += bytes.len() as u64;
        }
    }

    fn disk(&self) -> std::sync::MutexGuard<'_, RealFs> {
        self.disk.lock().unwrap()
    }
}

impl Vfs for CountingFs {
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
        self.disk().create_dir_all(dir)
    }
    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        self.disk().list(dir)
    }
    fn list_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        self.disk().list_dirs(dir)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let bytes = self.disk().read(path);
        self.count(path, &bytes);
        bytes
    }
    fn size(&self, path: &Path) -> io::Result<u64> {
        self.disk().size(path)
    }
    fn read_at(&self, path: &Path, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        let bytes = self.disk().read_at(path, offset, len);
        self.count(path, &bytes);
        bytes
    }
    fn append(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.disk().append(path, bytes)
    }
    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()> {
        self.disk().truncate(path, len)
    }
    fn sync_data(&mut self, path: &Path) -> io::Result<()> {
        self.disk().sync_data(path)
    }
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.disk().rename(from, to)
    }
    fn sync_dir(&mut self, dir: &Path) -> io::Result<()> {
        self.disk().sync_dir(dir)
    }
    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        self.disk().remove_file(path)
    }
    fn copy_permissions(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.disk().copy_permissions(from, to)
    }
    fn exists(&self, path: &Path) -> bool {
        self.disk().exists(path)
    }
}

/// The segments an opening touched, and whether the first segment and
/// the one before the last were each read inside one window.
type Reads = (Vec<String>, bool, bool);

/// What opening a ledger of `segments` segments of about 40 KB each, and
/// resuming from a snapshot cut at its last line, read: the names of the
/// segments it touched, and whether the reads of the first segment and of
/// the one before the last each stayed inside one window.
fn opening_reads(segments: usize) -> (Reads, Reads) {
    let tmp = tempfile::tempdir().unwrap();
    let (ledger_dir, snapshots) = (tmp.path().join("ledger"), tmp.path().join("snapshot"));
    let (mut ledger, _) = JsonlLedger::open(&ledger_dir, TimeMs::new(0)).unwrap();
    ledger.set_roll_bytes_for_test(40_000);
    let mut t = 0u64;
    while ledger_segments_at(&ledger_dir).unwrap().len() < segments {
        let wave: Vec<EventDraft> = (0..50)
            .map(|_| {
                t += 1;
                EventDraft {
                    run: RunId::CITY,
                    t: TimeMs::new(t),
                    who: "city".to_owned(),
                    addr: None,
                    kind: EventKind::GateChecked,
                    data: Payload::empty(),
                    ig: false,
                }
            })
            .collect();
        ledger.append_all(wave).unwrap();
    }
    let last_line = ledger.read_raw_lines().unwrap().pop().unwrap();
    let cut = Seq::new(ledger.position().value() - 1);
    drop(ledger);
    write_snapshot(
        &snapshots,
        &ChainSnapshot::cut(3, cut, &last_line, Vec::new()),
    )
    .unwrap();
    let names: Vec<String> = ledger_segments_at(&ledger_dir)
        .unwrap()
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    let counting = CountingFs::new();
    let (reopened, _) =
        JsonlLedger::open_with(Box::new(counting.clone()), &ledger_dir, TimeMs::new(1)).unwrap();
    let started = start_through(&counting, &ledger_dir, &snapshots, 3).unwrap();
    drop(reopened);
    assert!(matches!(started, SnapshotStart::Resume { .. }));

    let read = counting.read.lock().unwrap().clone();
    let (first, prior) = (&names[0], &names[names.len() - 2]);
    let observed = (
        read.keys().cloned().collect(),
        read[first] <= 4096,
        read[prior] <= 16 * 1024,
    );
    let expected = (
        vec![first.clone(), prior.clone(), names[names.len() - 1].clone()],
        true,
        true,
    );
    (observed, expected)
}

/// Before the first byte a served city reads the first line of its first
/// segment (the version probe), the last line of the segment before the
/// last (one window from its end), and the last segment; the segments
/// between are not read at all, so what an opening reads does not grow
/// with the history (sprawling-SPEC.md 8-122).
#[test]
fn an_opening_reads_one_line_of_the_segment_before_the_last_and_nothing_further_back() {
    let (short, long) = (opening_reads(4), opening_reads(8));

    assert_eq!((short.0, long.0), (short.1, long.1));
}
