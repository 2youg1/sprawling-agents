// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::error::MemoryError;
use kernel::{
    AxCode, EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs,
    chain_hash,
};
use std::fs;
use std::path::Path;
fn draft(kind: EventKind, t: u64) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(t),
        who: "city".to_string(),
        addr: None,
        kind,
        data: Payload::empty(),
        ig: false,
    }
}
fn verify_chain(lines: &[Vec<u8>]) {
    let mut prev = GENESIS_PREV;
    for (i, line) in lines.iter().enumerate() {
        let record = EventRecord::parse_line(line).unwrap();
        assert_eq!(record.prev(), prev, "prev broken at line {i}");
        assert_eq!(record.seq(), Seq::new(u64::try_from(i).unwrap()));
        prev = chain_hash(line);
    }
}

#[test]
fn empty_dir_opens_as_new_ledger_and_appends() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, report) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    assert!(report.recovered.is_none());
    let refs = ledger
        .append_all(vec![
            draft(EventKind::CityInitialized, 1),
            draft(EventKind::BuildingCreated, 2),
        ])
        .unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].seq(), Seq::new(0));
    assert_eq!(refs[1].seq(), Seq::new(1));
    let lines = ledger.read_raw_lines().unwrap();
    assert_eq!(lines.len(), 2);
    verify_chain(&lines);

    drop(ledger);
    let (mut reopened, report) = JsonlLedger::open(dir.path(), TimeMs::new(9)).unwrap();
    assert!(report.recovered.is_none(), "clean tail must not truncate");
    reopened
        .append_all(vec![draft(EventKind::RunStarted, 3)])
        .unwrap();
    let lines = reopened.read_raw_lines().unwrap();
    assert_eq!(lines.len(), 3);
    verify_chain(&lines);
}

#[cfg(feature = "conformance")]
#[test]
fn torn_tail_recovers_to_longest_valid_prefix_plus_log_truncated() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![
            draft(EventKind::CityInitialized, 1),
            draft(EventKind::BuildingCreated, 2),
            draft(EventKind::RunStarted, 3),
        ])
        .unwrap();
    let seg = only_segment(dir.path());
    drop(ledger);

    // Tear: cut the last line in half.
    let bytes = fs::read(&seg).unwrap();
    let cut = bytes.len() - 17;
    fs::write(&seg, &bytes[..cut]).unwrap();

    let (reopened, report) = JsonlLedger::open(dir.path(), TimeMs::new(77)).unwrap();
    let recovery = report.recovered.expect("must report the truncation");
    assert!(recovery.dropped_bytes > 0);
    let lines = reopened.read_raw_lines().unwrap();
    assert_eq!(lines.len(), 3, "two survivors plus log_truncated");
    verify_chain(&lines);
    let last = EventRecord::parse_line(&lines[2]).unwrap();
    assert_eq!(last.kind(), EventKind::LogTruncated);
    assert_eq!(last.t(), TimeMs::new(77), "time is the caller's parameter");
    assert_eq!(last.who(), "system");
}

#[test]
fn garbage_tail_without_newline_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![draft(EventKind::CityInitialized, 1)])
        .unwrap();
    let seg = only_segment(dir.path());
    drop(ledger);
    let mut bytes = fs::read(&seg).unwrap();
    bytes.extend_from_slice(b"{half of a torn write");
    fs::write(&seg, &bytes).unwrap();

    let (reopened, report) = JsonlLedger::open(dir.path(), TimeMs::new(5)).unwrap();
    assert!(report.recovered.is_some());
    let lines = reopened.read_raw_lines().unwrap();
    assert_eq!(lines.len(), 2);
    verify_chain(&lines);
}

#[test]
fn higher_version_fixture_is_refused_with_direction_and_path() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("ledger-v2");
    let before = fs::read(fixture.join("ledger-00000000000000000000.jsonl")).unwrap();

    let outcome = JsonlLedger::open(&fixture, TimeMs::new(0));
    let err = outcome.err().expect("v2 fixture must refuse to open");
    match &err {
        MemoryError::VersionAhead { path, v } => {
            assert_eq!(*v, 2);
            assert!(path.to_string_lossy().contains("ledger-v2"));
        }
        other => panic!("expected VersionAhead, got {other:?}"),
    }
    let ax = err.into_ax();
    assert_eq!(ax.code(), &AxCode::LogVersionUnsupported);
    assert!(ax.to_string().contains("newer"), "direction must be spoken");

    let after = fs::read(fixture.join("ledger-00000000000000000000.jsonl")).unwrap();
    assert_eq!(before, after, "browsing must never rewrite (A16)");
}
fn only_segment(dir: &Path) -> std::path::PathBuf {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    assert_eq!(files.len(), 1);
    files.remove(0)
}
