// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three properties of `crates/storage/spec/Jsonl/Preallocate.lean`,
//! checked on real segments: trailing zeros change no scan, a tear
//! before zeros is still truncated, and zeros before a line are not the
//! end of the segment.

use std::fs;
use std::path::{Path, PathBuf};

use kernel::{EventDraft, EventKind, Payload, RunId, TimeMs};
use proptest::prelude::*;

use crate::error::StorageError;
use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
use crate::jsonl::JsonlLedger;

fn draft(t: u64) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(t),
        who: "city".to_string(),
        addr: None,
        kind: EventKind::GateChecked,
        data: Payload::empty(),
        ig: false,
    }
}

/// The lines (each with its `\n`) of a one-segment ledger of `count`
/// records, and that segment's file name.
fn written(count: u64) -> (Vec<Vec<u8>>, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("ledger");
    let (mut ledger, _) = JsonlLedger::open(&dir, TimeMs::new(0)).unwrap();
    ledger.append_all((0..count).map(draft).collect()).unwrap();
    let lines = ledger
        .read_raw_lines()
        .unwrap()
        .into_iter()
        .map(|mut line| {
            line.push(b'\n');
            line
        })
        .collect();
    let name = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .find(|name| name.to_string_lossy().ends_with(".jsonl"))
        .unwrap();
    (lines, PathBuf::from(name))
}

/// A ledger directory under `root` whose one segment holds `bytes`.
fn segment_of(root: &Path, name: &Path, bytes: &[u8]) -> PathBuf {
    let dir = root.join("ledger");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(name), bytes).unwrap();
    dir
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

    /// `trailing_zeros_change_no_scan`, and the writer position is the
    /// length after the zeros: the next wave lands where the last record
    /// ended, so the ledger reopens whole with nothing recovered.
    #[test]
    fn trailing_zeros_change_no_scan(count in 1u64..5, zeros in 1usize..9000) {
        let (lines, name) = written(count);
        let root = tempfile::tempdir().unwrap();
        let mut bytes = lines.concat();
        bytes.resize(bytes.len().saturating_add(zeros), 0);
        let dir = segment_of(root.path(), &name, &bytes);

        let (mut ledger, report) = JsonlLedger::open(&dir, TimeMs::new(9)).unwrap();
        prop_assert!(report.recovered.is_none(), "trailing zeros were read as a tear");
        prop_assert_eq!(ledger.position().value(), count);
        ledger.append_all(vec![draft(count)]).unwrap();
        drop(ledger);

        let (reopened, report) = JsonlLedger::open(&dir, TimeMs::new(10)).unwrap();
        prop_assert!(report.recovered.is_none(), "the wave after the zeros was torn off");
        prop_assert_eq!(reopened.read_raw_lines().unwrap().len(), lines.len().saturating_add(1));
    }

    /// `a_tear_before_zeros_is_still_truncated`: the records before the
    /// tear survive, and only they.
    #[test]
    fn a_tear_before_zeros_is_still_truncated(
        count in 1u64..5,
        torn in 1usize..40,
        zeros in 0usize..9000,
    ) {
        let (lines, name) = written(count.saturating_add(1));
        let root = tempfile::tempdir().unwrap();
        let (kept, last) = lines.split_at(lines.len().saturating_sub(1));
        let mut bytes = kept.concat();
        bytes.extend(last.concat().iter().take(torn));
        bytes.resize(bytes.len().saturating_add(zeros), 0);
        let dir = segment_of(root.path(), &name, &bytes);

        let (ledger, report) = JsonlLedger::open(&dir, TimeMs::new(9)).unwrap();
        prop_assert!(report.recovered.is_some(), "the tear was kept");
        let survived = ledger.read_raw_lines().unwrap();
        prop_assert_eq!(survived.len(), kept.len().saturating_add(1), "records, then log_truncated");
        prop_assert_eq!(
            survived.iter().take(kept.len()).map(|l| [l.as_slice(), b"\n"].concat()).collect::<Vec<_>>(),
            kept.to_vec()
        );
    }

    /// `zeros_before_a_line_are_not_the_end`: a record after a run of
    /// zeros makes the zeros damage inside the segment, and open refuses
    /// instead of truncating that record away.
    #[test]
    fn zeros_before_a_line_are_not_the_end(count in 1u64..5, zeros in 1usize..9000) {
        let (lines, name) = written(count.saturating_add(1));
        let root = tempfile::tempdir().unwrap();
        let (kept, last) = lines.split_at(lines.len().saturating_sub(1));
        let mut bytes = kept.concat();
        bytes.resize(bytes.len().saturating_add(zeros), 0);
        bytes.extend(last.concat());
        let dir = segment_of(root.path(), &name, &bytes);

        let opened = JsonlLedger::open(&dir, TimeMs::new(9));
        prop_assert!(
            matches!(opened, Err(StorageError::Envelope { .. })),
            "open answered {:?} instead of refusing",
            opened.map(|(_, report)| report.recovered.is_some())
        );
        prop_assert_eq!(fs::read(dir.join(&name)).unwrap(), bytes, "a refused open repaired the segment");
    }
}

/// The ledger behind `fs`, reopened with the preallocating arm until an
/// opening survives the power cut the plan may still hold.
fn reopened(fs: &FaultFs, dir: &Path, roll: u64) -> JsonlLedger {
    let (mut ledger, _) = JsonlLedger::open_preallocated(fs.clone(), dir, TimeMs::new(9))
        .or_else(|_| JsonlLedger::open_preallocated(fs.clone(), dir, TimeMs::new(9)))
        .unwrap();
    ledger.roll_bytes = roll;
    ledger
}

/// Every record of the ledger behind `ledger`, each with its seq, the
/// chain checked line by line. A cut the waves never reached may fall on
/// this read, so it is read again once.
fn chained(ledger: &JsonlLedger) -> Vec<u64> {
    let mut prev = kernel::GENESIS_PREV;
    ledger
        .read_raw_lines()
        .or_else(|_| ledger.read_raw_lines())
        .unwrap()
        .iter()
        .map(|line| {
            let record = kernel::EventRecord::parse_line(line).unwrap();
            assert_eq!(record.prev(), prev);
            prev = kernel::ledger::chain_hash(line);
            record.seq().value()
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// Derived from `a_record_written_at_the_stripped_end_resumes_the_segment`
    /// and `writer_position_is_the_stripped_end`: with the preallocating
    /// arm (storage D31), a writer that reopens between any two waves, or
    /// after power is lost at any operation, resumes at the end of its
    /// records. Every wave that answered `Ok` reads back, the seqs run
    /// without a gap and the chain holds; a log_truncated line appears
    /// only where a cut tore a wave, and the last segment keeps its
    /// preallocated length when nothing was torn.
    #[test]
    fn a_preallocated_writer_resumes_after_every_reopen_and_every_cut(
        waves in proptest::collection::vec(1u64..4, 1..6),
        roll in 300u64..3000,
        cut in proptest::option::of(1u64..80),
    ) {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("ledger");
        let fs = FaultFs::new(FaultPlan {
            cut_at_op: cut,
            cut_on_write: None,
            torn_tail: TornTail::KeepBytes(7),
        });
        let mut ledger = reopened(&fs, &dir, roll);
        let mut acknowledged = 0u64;
        let mut t = 0u64;
        for wave in waves {
            let drafts = (0..wave).map(|i| draft(t.saturating_add(i))).collect();
            t = t.saturating_add(wave);
            // A refused wave is what the cut tore; it owes nothing back.
            if let Ok(refs) = ledger.append_all(drafts) {
                acknowledged = acknowledged.saturating_add(u64::try_from(refs.len()).unwrap());
            }
            drop(ledger);
            ledger = reopened(&fs, &dir, roll);
        }

        let seqs = chained(&ledger);
        let expected: Vec<u64> = (0..u64::try_from(seqs.len()).unwrap()).collect();
        let read = u64::try_from(seqs.len()).unwrap();
        prop_assert_eq!(&seqs, &expected);
        prop_assert!(read >= acknowledged);
        if cut.is_none() {
            prop_assert_eq!(read, acknowledged);
            let last = crate::vfs::Vfs::list(&fs, &dir)
                .unwrap()
                .into_iter()
                .rfind(|path| super::super::ledger::is_segment(path))
                .unwrap();
            prop_assert!(crate::vfs::Vfs::size(&fs, &last).unwrap() >= roll);
        }
    }
}
