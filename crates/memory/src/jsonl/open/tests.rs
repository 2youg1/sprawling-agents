// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::error::MemoryError;
use kernel::ledger::chain_hash;
use kernel::{
    AxCode, EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs,
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
    assert_eq!(
        last.who(),
        kernel::event::Who::City.as_str(),
        "opening the ledger is the city's own work, and the actor vocabulary has \
         one home"
    );
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
    let segment = "ledger-00000000000000000000.jsonl";
    let before = fs::read(fixture.join(segment)).unwrap();
    // `open` takes a lock beside the directory it opens, so it opens a copy
    // and the source tree stays untouched.
    let scratch = tempfile::tempdir().unwrap();
    let copy = scratch.path().join("ledger-v2");
    fs::create_dir(&copy).unwrap();
    fs::copy(fixture.join(segment), copy.join(segment)).unwrap();

    let outcome = JsonlLedger::open(&copy, TimeMs::new(0));
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

    let after = fs::read(copy.join(segment)).unwrap();
    assert_eq!(before, after, "browsing must never rewrite (A16)");
    assert!(
        !fixture.with_extension("lock").exists(),
        "a test must not write into the source tree"
    );
}
/// A `v` below the first version anybody wrote is refused for being
/// that, wherever it sits. It used to be refused as a broken chain when
/// it was line one, and silently truncated away when it was not, which
/// told a person to restore a chain that was never damaged.
#[test]
fn a_line_below_the_first_version_is_refused_for_its_version_not_its_chain() {
    for unversioned_line in [0usize, 1usize] {
        let dir = tempfile::tempdir().unwrap();
        let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger
            .append_all(vec![
                draft(EventKind::CityInitialized, 1),
                draft(EventKind::BuildingCreated, 2),
            ])
            .unwrap();
        let seg = only_segment(dir.path());
        drop(ledger);

        // Same byte count, so every prev in the chain still holds: what
        // the line declares is the only thing that changed.
        let text = fs::read_to_string(&seg).unwrap();
        let rewritten: Vec<String> = text
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == unversioned_line {
                    line.replacen("\"v\":1", "\"v\":0", 1)
                } else {
                    line.to_owned()
                }
            })
            .collect();
        assert!(
            rewritten[unversioned_line].contains("\"v\":0"),
            "the fixture must actually declare v0"
        );
        fs::write(
            &seg,
            format!(
                "{}
",
                rewritten.join(
                    "
"
                )
            ),
        )
        .unwrap();

        let err = JsonlLedger::open(dir.path(), TimeMs::new(3))
            .err()
            .expect("a v0 line must refuse to open");
        let MemoryError::Envelope { line, source, .. } = &err else {
            panic!("expected Envelope, got {err:?}");
        };
        assert_eq!(*line, u64::try_from(unversioned_line).unwrap() + 1);
        let said = source.to_string();
        assert!(
            said.contains("v0"),
            "the refusal must name the version: {said}"
        );
        assert!(
            !said.contains("chain"),
            "a version refusal must not blame the chain: {said}"
        );
    }
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

/// A city's Ledger has one writer. The lock is the operating system's
/// and it belongs to an open handle, not to a process, so a second
/// `open` in this process meets exactly what a second process meets:
/// a refusal before anything is read or repaired. When the first writer
/// is dropped the lock goes with it.
#[test]
fn a_second_writer_of_a_city_is_refused_until_the_first_lets_go() {
    let city = tempfile::tempdir().unwrap();
    let dir = kernel::layout::CityLayout::new(city.path()).ledger();
    let (mut first, _) = JsonlLedger::open(&dir, TimeMs::new(0)).unwrap();
    first
        .append_all(vec![draft(EventKind::CityInitialized, 0)])
        .unwrap();

    let second = JsonlLedger::open(&dir, TimeMs::new(1))
        .err()
        .map(|refused| refused.into_ax().code().as_str());
    assert_eq!(
        second,
        Some("E_LEDGER_HELD"),
        "a second writer opened a city whose ledger another writer holds"
    );

    drop(first);
    let (reopened, _) = JsonlLedger::open(&dir, TimeMs::new(2)).unwrap();
    assert_eq!(
        reopened.position(),
        Seq::new(1),
        "the refusal wrote nothing"
    );
}

/// The lock is named from the ledger directory alone, so a directory
/// that sits in no city is held by one writer just as a city's is.
#[test]
fn a_second_writer_of_a_ledger_outside_any_city_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("history");
    let (_first, _) = JsonlLedger::open(&dir, TimeMs::new(0)).unwrap();

    let second = JsonlLedger::open(&dir, TimeMs::new(1))
        .err()
        .map(|refused| refused.into_ax().code().as_str());
    assert_eq!(
        second,
        Some("E_LEDGER_HELD"),
        "a second writer opened a ledger another writer holds"
    );
}

/// Two writers that each continued the same `prev` leave a fork: every line after the
/// fork point is a complete, lawful record. Reopening must refuse and leave all of them
/// on disk, because truncating would delete the other writer's history.
#[test]
fn a_fork_after_the_first_line_is_refused_and_keeps_every_line() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![
            draft(EventKind::CityInitialized, 1),
            draft(EventKind::BuildingCreated, 2),
        ])
        .unwrap();
    let seg = only_segment(dir.path());
    let fork_point = fs::read(&seg).unwrap();
    ledger
        .append_all(vec![draft(EventKind::BuildingCreated, 3)])
        .unwrap();
    drop(ledger);
    let first_writer = fs::read(&seg).unwrap();

    // The second writer continues from the same fork point.
    fs::write(&seg, &fork_point).unwrap();
    let (mut second, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    second
        .append_all(vec![
            draft(EventKind::BuildingCreated, 4),
            draft(EventKind::BuildingCreated, 5),
        ])
        .unwrap();
    drop(second);
    let mut forked = first_writer;
    forked.extend_from_slice(&fs::read(&seg).unwrap()[fork_point.len()..]);
    fs::write(&seg, &forked).unwrap();

    let outcome = JsonlLedger::open(dir.path(), TimeMs::new(9));
    assert_eq!(
        fs::read(&seg).unwrap(),
        forked,
        "reopening must not delete lawful history"
    );
    let err = outcome.err().expect("a forked ledger must refuse to open");
    let MemoryError::Envelope { line, .. } = &err else {
        panic!("expected Envelope, got {err:?}");
    };
    assert_eq!(*line, 4, "the refusal names the first line after the fork");
}

/// A line from a newer vocabulary that marks itself ignorable is lawful
/// history: replay reads it and chains past it, so open must keep it and
/// continue the chain after it rather than truncate it as tail damage.
#[test]
fn an_ignorable_line_from_a_newer_vocabulary_is_kept_and_chained() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![draft(EventKind::CityInitialized, 1)])
        .unwrap();
    let genesis = ledger.read_raw_lines().unwrap().remove(0);
    drop(ledger);
    let future = format!(
        "{{\"v\":1,\"run\":\"00000000-0000-0000-0000-000000000000\",\"seq\":1,\
         \"prev\":\"{}\",\"t\":0,\"who\":\"city\",\"kind\":\"kind_from_the_future\",\
         \"data\":{{}},\"ig\":true}}\n",
        chain_hash(&genesis)
    );
    let segment = only_segment(dir.path());
    let mut bytes = fs::read(&segment).unwrap();
    bytes.extend_from_slice(future.as_bytes());
    fs::write(&segment, &bytes).unwrap();

    let (mut reopened, report) = JsonlLedger::open(dir.path(), TimeMs::new(9)).unwrap();
    assert!(
        report.recovered.is_none(),
        "a lawful line is not tail damage"
    );
    assert_eq!(reopened.position(), Seq::new(2));
    reopened
        .append_all(vec![draft(EventKind::RunStarted, 3)])
        .unwrap();
    assert_eq!(reopened.read_raw_lines().unwrap().len(), 3);
}

/// The version probe reads the first line, not the first segment: a
/// single-segment ledger is read once on open (by tail recovery), not
/// twice.
#[test]
fn opening_a_single_segment_ledger_reads_it_once() {
    use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: TornTail::None,
    });
    let dir = Path::new("l");
    let (mut ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir, TimeMs::new(0)).unwrap();
    let drafts = (0..2_000)
        .map(|t| draft(EventKind::RunStarted, t))
        .collect();
    ledger.append_all(drafts).unwrap();
    let segment_len: u64 = ledger
        .read_raw_lines()
        .unwrap()
        .iter()
        .map(|line| u64::try_from(line.len()).unwrap() + 1)
        .sum();
    drop(ledger);

    let before = fs.bytes_read();
    JsonlLedger::open_faulty(fs.clone(), dir, TimeMs::new(1)).unwrap();
    let moved = fs.bytes_read() - before;
    assert!(
        moved.saturating_mul(10) < segment_len.saturating_mul(12),
        "opening a {segment_len}-byte ledger read {moved} bytes"
    );
}
