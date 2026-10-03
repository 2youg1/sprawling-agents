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
