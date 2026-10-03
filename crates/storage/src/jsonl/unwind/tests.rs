// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]

use kernel::ledger::chain_hash;
use kernel::{EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs};

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
use crate::jsonl::JsonlLedger;
use crate::real_fs::RealFs;
use crate::vfs::Vfs;

fn draft(who: &str, t: u64) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(t),
        who: who.to_string(),
        addr: None,
        kind: EventKind::GateChecked,
        data: Payload::empty(),
        ig: false,
    }
}

/// A write that dies half-way (a full disk, a pulled cable) leaves torn
/// bytes at the segment's tail. The process lives on and appends again;
/// the next wave must land where the last durable line ended, so the
/// ledger reopens as one unbroken chain with nothing recovered.
#[test]
fn a_failed_write_leaves_no_bytes_the_next_wave_builds_on() {
    let dir = tempfile::tempdir().unwrap();
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: Some("doomed"),
        torn_tail: TornTail::KeepBytes(7),
    });
    let (mut ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir.path(), TimeMs::new(0)).unwrap();
    ledger.append_all(vec![draft("city", 1)]).unwrap();
    assert!(ledger.append_all(vec![draft("doomed", 2)]).is_err());
    ledger.append_all(vec![draft("city", 3)]).unwrap();
    drop(ledger);

    let (reopened, report) = JsonlLedger::open_faulty(fs, dir.path(), TimeMs::new(9))
        .expect("the ledger reopens after a failed write");
    let whos: Vec<(Seq, String)> = reopened
        .read_raw_lines()
        .unwrap()
        .iter()
        .scan(GENESIS_PREV, |prev, line| {
            let record = EventRecord::parse_line(line).unwrap();
            assert_eq!(
                record.prev(),
                *prev,
                "the chain breaks before seq {:?}",
                record.seq()
            );
            *prev = chain_hash(line);
            Some((record.seq(), record.who().to_string()))
        })
        .collect();
    assert!(report.recovered.is_none(), "reopen had to cut torn bytes");
    assert_eq!(
        whos,
        vec![
            (Seq::new(0), "city".to_string()),
            (Seq::new(1), "city".to_string())
        ]
    );
}

/// A restore that cut the torn bytes off makes the cut durable: power
/// lost after the failed wave and before the next one reopens the ledger
/// with nothing to recover, on every platform the barrier runs on.
#[test]
fn a_restored_tail_survives_a_power_cut() {
    let dir = tempfile::tempdir().unwrap();
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: Some("doomed"),
        torn_tail: TornTail::KeepBytes(7),
    });
    let (mut ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir.path(), TimeMs::new(0)).unwrap();
    ledger.append_all(vec![draft("city", 1)]).unwrap();
    assert!(ledger.append_all(vec![draft("doomed", 2)]).is_err());
    drop(ledger);
    fs.power_cut();

    let (reopened, report) = JsonlLedger::open_faulty(fs, dir.path(), TimeMs::new(9)).unwrap();
    assert!(
        report.recovered.is_none(),
        "the restore's truncation was lost with the power"
    );
    assert_eq!(reopened.position(), Seq::new(1));
}

/// What the next filesystem call of its kind does instead of succeeding.
#[derive(Clone, Copy, Debug)]
enum Fault {
    /// An append that writes this many bytes of its line, then fails.
    Tear(usize),
    /// A barrier that fails after the bytes reached the file.
    Unsynced,
    /// A truncate or a remove that is refused: the restore fails.
    RefuseRestore,
}

/// `RealFs`, with the faults of one trace queued in the order the
/// handle meets them.
struct Scripted {
    real: RealFs,
    faults: Arc<Mutex<VecDeque<Fault>>>,
}

impl Scripted {
    fn take(&self, wanted: fn(Fault) -> bool) -> Option<Fault> {
        let mut faults = self.faults.lock().unwrap();
        let hit = faults.front().copied().filter(|fault| wanted(*fault));
        if hit.is_some() {
            faults.pop_front();
        }
        hit
    }
}

fn refused() -> io::Error {
    io::Error::other("scripted fault")
}

impl Vfs for Scripted {
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
        self.real.create_dir_all(dir)
    }
    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        self.real.list(dir)
    }
    fn list_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        self.real.list_dirs(dir)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.real.read(path)
    }
    fn size(&self, path: &Path) -> io::Result<u64> {
        self.real.size(path)
    }
    fn read_at(&self, path: &Path, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        self.real.read_at(path, offset, len)
    }
    fn append(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        match self.take(|fault| matches!(fault, Fault::Tear(_))) {
            Some(Fault::Tear(kept)) => {
                self.real.append(path, &bytes[..kept])?;
                Err(refused())
            }
            _ => self.real.append(path, bytes),
        }
    }
    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()> {
        match self.take(|fault| matches!(fault, Fault::RefuseRestore)) {
            Some(_) => Err(refused()),
            None => self.real.truncate(path, len),
        }
    }
    fn sync_data(&mut self, path: &Path) -> io::Result<()> {
        match self.take(|fault| matches!(fault, Fault::Unsynced)) {
            Some(_) => Err(refused()),
            None => self.real.sync_data(path),
        }
    }
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.real.rename(from, to)
    }
    fn sync_dir(&mut self, dir: &Path) -> io::Result<()> {
        self.real.sync_dir(dir)
    }
    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        match self.take(|fault| matches!(fault, Fault::RefuseRestore)) {
            Some(_) => Err(refused()),
            None => self.real.remove_file(path),
        }
    }
    fn copy_permissions(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.real.copy_permissions(from, to)
    }
    fn exists(&self, path: &Path) -> bool {
        self.real.exists(path)
    }
}

/// One wave of a trace: the faults it meets, and whether the handle
/// answers `Ok`.
struct Wave {
    faults: &'static [Fault],
    answers_ok: bool,
}

const fn wave(faults: &'static [Fault], answers_ok: bool) -> Wave {
    Wave { faults, answers_ok }
}

use Fault::{RefuseRestore, Tear, Unsynced};

/// Trace vectors over `Step` from `crates/storage/spec/Jsonl/Barrier.lean`
/// (`unwind_sound`, `answered_survives_reopen`). A failed wave is the
/// model's `wave tore` or `wave unsynced` followed by the restore the
/// handle runs at once (`unwind done`, or `unwind refused` when the
/// restore meets `RefuseRestore`); a wave after a refused restore runs
/// the pending restore first, the model's `unwind` before its `wave`.
const TRACES: &[&[Wave]] = &[
    &[
        wave(&[], true),
        wave(&[Tear(7)], false),
        wave(&[], true),
        wave(&[], true),
    ],
    &[wave(&[], true), wave(&[Unsynced], false), wave(&[], true)],
    &[
        wave(&[], true),
        wave(&[Tear(7), RefuseRestore], false),
        wave(&[], true),
    ],
    &[
        wave(&[], true),
        wave(&[Tear(7), RefuseRestore], false),
        wave(&[RefuseRestore], false),
        wave(&[], true),
    ],
    &[wave(&[], true), wave(&[Unsynced, RefuseRestore], false)],
    &[wave(&[], true), wave(&[Tear(7), RefuseRestore], false)],
    &[
        wave(&[Unsynced], false),
        wave(&[Tear(3), RefuseRestore], false),
        wave(&[], true),
        wave(&[Unsynced, RefuseRestore], false),
        wave(&[], true),
    ],
];

/// Every seq a handle answered `Ok` for is in the ledger that reopens
/// after the trace, at its place in one unbroken chain.
#[test]
fn every_answered_seq_survives_reopen_on_every_trace() {
    for (index, trace) in TRACES.iter().enumerate() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("ledger");
        let faults = Arc::new(Mutex::new(VecDeque::new()));
        let vfs = Scripted {
            real: RealFs::new(),
            faults: Arc::clone(&faults),
        };
        let (mut ledger, _) = JsonlLedger::open_with(Box::new(vfs), &dir, TimeMs::new(0)).unwrap();
        let mut answered = Vec::new();
        for (t, step) in trace.iter().enumerate() {
            faults.lock().unwrap().extend(step.faults.iter().copied());
            let refs = ledger.append_all(vec![draft("city", u64::try_from(t).unwrap())]);
            assert_eq!(
                refs.is_ok(),
                step.answers_ok,
                "trace {index}, wave {t}: {refs:?}"
            );
            answered.extend(refs.into_iter().flatten().map(|r| r.seq()));
            assert!(
                faults.lock().unwrap().is_empty(),
                "trace {index}, wave {t} left a fault unmet"
            );
        }
        drop(ledger);

        let (reopened, _) = JsonlLedger::open(&dir, TimeMs::new(99)).unwrap();
        let kept: Vec<Seq> = reopened
            .read_raw_lines()
            .unwrap()
            .iter()
            .map(|line| EventRecord::parse_line(line).unwrap().seq())
            .collect();
        for seq in &answered {
            assert!(
                kept.contains(seq),
                "trace {index}: seq {seq:?} was answered and lost"
            );
        }
    }
}
