// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use crate::error::MemoryError;
use kernel::{B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use std::path::Path;
fn write_ledger(dir: &Path, count: u64) -> Vec<Vec<u8>> {
    write_ledger_rolling(dir, count, count.max(1))
}

/// Writes `count` records into segments that roll every
/// `per_segment` lines, and returns each line in seq order.
fn write_ledger_rolling(dir: &Path, count: u64, per_segment: u64) -> Vec<Vec<u8>> {
    let run = RunId::from_bytes([7u8; 16]);
    let mut lines = Vec::new();
    let mut blob = Vec::new();
    let mut first_seq = 0u64;
    let mut prev = B3Hash::digest(b"");
    for i in 0..count {
        let draft = EventDraft {
            run,
            t: TimeMs::new(i),
            who: "tester".to_owned(),
            addr: None,
            kind: EventKind::RunStarted,
            data: Payload::new(serde_json::Map::new()).unwrap(),
            ig: false,
        };
        let record = EventRecord::from_draft(draft, Seq::new(i), prev);
        let line = record.canonical_line().unwrap();
        prev = B3Hash::digest(&line);
        blob.extend_from_slice(&line);
        blob.push(b'\n');
        lines.push(line);
        let next = i.saturating_add(1);
        if next.checked_rem(per_segment) == Some(0) && next < count {
            std::fs::write(dir.join(format!("ledger-{first_seq:020}.jsonl")), &blob).unwrap();
            blob.clear();
            first_seq = next;
        }
    }
    std::fs::write(dir.join(format!("ledger-{first_seq:020}.jsonl")), &blob).unwrap();
    lines
}

fn write_interleaved(dir: &Path, runs: &[RunId], count: u64) -> Vec<(RunId, Vec<Seq>)> {
    let mut blob = Vec::new();
    let mut prev = B3Hash::digest(b"");
    let mut owned: Vec<(RunId, Vec<Seq>)> = runs.iter().map(|run| (*run, Vec::new())).collect();
    for i in 0..count {
        let slot = usize::try_from(i)
            .unwrap()
            .checked_rem(runs.len())
            .expect("at least one run to cycle through");
        let run = runs[slot];
        let draft = EventDraft {
            run,
            t: TimeMs::new(i),
            who: "tester".to_owned(),
            addr: None,
            kind: EventKind::RunStarted,
            data: Payload::new(serde_json::Map::new()).unwrap(),
            ig: false,
        };
        let record = EventRecord::from_draft(draft, Seq::new(i), prev);
        let line = record.canonical_line().unwrap();
        prev = B3Hash::digest(&line);
        blob.extend_from_slice(&line);
        blob.push(b'\n');
        owned[slot].1.push(Seq::new(i));
    }
    std::fs::write(dir.join("ledger-00000000000000000000.jsonl"), &blob).unwrap();
    owned
}

/// The whole point of the run index: one session's lines are found
/// without reading anybody else's. The assertion is on the seqs the
/// index hands back, because that set *is* what the reader will open
/// the segment for - a walk that returned the other runs' seqs and
/// left the filtering to the caller would have read them all.
#[test]
fn rebuild_finds_every_line_and_seeking_returns_it_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let lines = write_ledger(tmp.path(), 5);
    let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(index.len(), 5);
    assert_eq!(index.tail_seq(), Some(Seq::new(4)));
    let mut reader = index.reader(tmp.path());
    for (i, expected) in lines.iter().enumerate() {
        let got = reader.line_at(Seq::new(u64::try_from(i).unwrap())).unwrap();
        assert_eq!(&got, expected, "line {i} seeks back verbatim");
    }
}

#[test]
fn a_fresh_cache_is_believed_and_a_stale_one_is_silently_rebuilt() {
    let tmp = tempfile::tempdir().unwrap();
    write_ledger(tmp.path(), 3);
    let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    index.persist(tmp.path()).unwrap();
    // Fresh: the cache round-trips into an identical map.
    let loaded = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded.tail_seq(), Some(Seq::new(2)));
    // Stale: growing the ledger without refreshing the cache must
    // not yield the old answer.
    write_ledger(tmp.path(), 6);
    let after = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(after.len(), 6, "byte drift forces a rebuild");
    assert_eq!(after.tail_seq(), Some(Seq::new(5)));
}

#[test]
fn a_corrupt_cache_rebuilds_instead_of_reporting() {
    let tmp = tempfile::tempdir().unwrap();
    write_ledger(tmp.path(), 4);
    std::fs::write(
        tmp.path().join(crate::index::cache::CACHE_NAME),
        format!("{} garbage\nnot a row\n", super::super::cache::CACHE_MAGIC),
    )
    .unwrap();
    let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(index.len(), 4, "a disposable artifact never reports");
}

#[test]
fn a_torn_tail_is_skipped_and_the_intact_prefix_still_indexes() {
    let tmp = tempfile::tempdir().unwrap();
    write_ledger(tmp.path(), 3);
    let path = tmp.path().join("ledger-00000000000000000000.jsonl");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.extend_from_slice(b"{\"seq\":99,\"partial\"");
    std::fs::write(&path, &bytes).unwrap();
    let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(index.len(), 3);
    assert_eq!(index.tail_seq(), Some(Seq::new(2)));
}

/// The refresh reads what was appended and nothing else, and a
/// segment that lost its tail is not patched but rebuilt.
#[test]
fn a_resident_index_folds_what_arrived_and_rebuilds_what_was_truncated() {
    let tmp = tempfile::tempdir().unwrap();
    write_ledger(tmp.path(), 3);
    let mut index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(index.len(), 3);

    // Nothing moved: refreshing is a no-op, and the map is unchanged.
    index.refresh(tmp.path()).unwrap();
    assert_eq!(index.len(), 3);

    // Appended: the new lines land, and the old offsets still read.
    let lines = write_ledger(tmp.path(), 7);
    index.refresh(tmp.path()).unwrap();
    assert_eq!(index.len(), 7);
    assert_eq!(index.tail_seq(), Some(Seq::new(6)));
    let mut reader = index.reader(tmp.path());
    for (i, expected) in lines.iter().enumerate() {
        let got = reader.line_at(Seq::new(u64::try_from(i).unwrap())).unwrap();
        assert_eq!(&got, expected, "line {i} still reads after a refresh");
    }

    // Truncated: the offsets held for that segment describe bytes
    // that are gone, so the whole map is rebuilt rather than patched.
    let path = tmp.path().join("ledger-00000000000000000000.jsonl");
    let bytes = std::fs::read(&path).unwrap();
    let keep = bytes.len() / 2;
    std::fs::write(&path, bytes.get(..keep).unwrap()).unwrap();
    index.refresh(tmp.path()).unwrap();
    assert!(index.len() < 7, "a shrunken segment rebuilds");
    let held = index.len();
    let mut reader = index.reader(tmp.path());
    for i in 0..held {
        reader
            .line_at(Seq::new(u64::try_from(i).unwrap()))
            .unwrap_or_else(|err| panic!("seq {i} unreadable after rebuild: {err}"));
    }
}

/// A refresh reads the appended tail, not the segment that holds it.
///
/// The assertion is on bytes, because that is the whole difference: an
/// index that lifted the segment and then sliced its tail folds exactly
/// the same records and answers exactly the same queries, while paying
/// the length of the ledger on every query that follows an append.
#[test]
fn a_refresh_reads_the_appended_tail_rather_than_the_segment() {
    let tmp = tempfile::tempdir().unwrap();
    let before = write_ledger(tmp.path(), 400);
    let mut index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    let path = tmp.path().join("ledger-00000000000000000000.jsonl");
    let segment_bytes = std::fs::metadata(&path).unwrap().len();

    // Nothing grew, so nothing is opened at all.
    assert_eq!(index.refresh(tmp.path()).unwrap(), Refreshed::Unchanged);

    let after = write_ledger(tmp.path(), 401);
    let appended = u64::try_from(
        after
            .get(400)
            .map(|line| line.len().saturating_add(1))
            .unwrap(),
    )
    .unwrap();
    let cost = index.refresh(tmp.path()).unwrap();
    assert_eq!(
        cost,
        Refreshed::Appended {
            bytes_read: appended
        },
        "one appended record costs one record's bytes, not {segment_bytes}"
    );
    assert!(
        appended.saturating_mul(10) < segment_bytes,
        "the fixture must be large enough for the two readings to differ"
    );
    eprintln!("index_refresh_read: {appended} B read, segment {segment_bytes} B");
    assert_eq!(index.len(), 401);
    assert_eq!(index.tail_seq(), Some(Seq::new(400)));

    // The offsets folded before the append still name the right lines.
    let mut reader = index.reader(tmp.path());
    assert_eq!(
        &reader.line_at(Seq::new(0)).unwrap(),
        before.first().unwrap()
    );
    assert_eq!(
        &reader.line_at(Seq::new(400)).unwrap(),
        after.get(400).unwrap()
    );

    // A segment that lost its tail is rebuilt, and says so.
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, bytes.get(..bytes.len() / 2).unwrap()).unwrap();
    assert_eq!(index.refresh(tmp.path()).unwrap(), Refreshed::Rebuilt);
}

/// Writes `count` records that cycle through `runs`, so no run owns a
/// contiguous stretch of the ledger. Returns the seqs each run wrote.
#[test]
fn the_run_map_survives_a_refresh_and_a_cache_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let mine = RunId::from_bytes([1u8; 16]);
    let yours = RunId::from_bytes([2u8; 16]);
    write_interleaved(tmp.path(), &[mine, yours], 4);
    let mut index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(index.run_seqs_before(mine, None).count(), 2);

    // Appended: the refresh folds the new lines into both maps.
    write_interleaved(tmp.path(), &[mine, yours], 10);
    index.refresh(tmp.path()).unwrap();
    assert_eq!(index.run_seqs_before(mine, None).count(), 5);

    // Persisted and read back: a cache that carried offsets but not
    // runs would answer this with nothing.
    index.persist(tmp.path()).unwrap();
    let loaded = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(loaded.len(), 10, "the cache was believed, not rebuilt");
    assert_eq!(loaded.run_seqs_before(mine, None).count(), 5);
}

/// A line whose `run` cannot be read still belongs in the seq map:
/// an index over a damaged ledger is what a repair path needs, and
/// dropping the line would hide it from every reader.
#[test]
fn a_line_with_no_readable_run_is_still_indexed_by_seq() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("ledger-00000000000000000000.jsonl");
    std::fs::write(&path, b"{\"seq\":0,\"run\":\"not-a-uuid\"}\n{\"seq\":1}\n").unwrap();
    let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    assert_eq!(index.len(), 2, "both lines are locatable");
    assert_eq!(index.tail_seq(), Some(Seq::new(1)));
    assert_eq!(
        index.run_seqs_before(RunId::CITY, None).count(),
        0,
        "an unreadable run joins no run rather than joining the nil one"
    );
}

#[test]
fn a_missing_seq_is_a_caller_error_not_a_corrupt_ledger() {
    let tmp = tempfile::tempdir().unwrap();
    write_ledger(tmp.path(), 2);
    let index = LedgerIndex::load_or_rebuild(tmp.path()).unwrap();
    let err = match index.reader(tmp.path()).line_at(Seq::new(77)) {
        Err(err) => err,
        Ok(_) => panic!("an absent seq must not read"),
    };
    assert!(matches!(err, MemoryError::SeqMissing { seq: 77 }));
}

/// What rebuilding a view costs, counted rather than timed: every
/// record is folded once and every byte of the ledger is read once, so
/// a rebuild that starts re-reading segments shows up here as read
/// amplification rather than as a slow afternoon.
#[test]
fn a_view_rebuild_folds_every_record_and_reads_the_ledger_once() {
    let dir = tempfile::tempdir().unwrap();
    let fs = crate::fault_fs::FaultFs::new(crate::fault_fs::FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: crate::fault_fs::TornTail::None,
    });
    let (mut ledger, _) =
        crate::JsonlLedger::open_faulty(fs.clone(), dir.path(), TimeMs::new(0)).unwrap();
    let run = RunId::from_bytes([5u8; 16]);
    let drafts: Vec<EventDraft> = (0..401)
        .map(|i| EventDraft {
            run,
            t: TimeMs::new(i),
            who: "tester".to_owned(),
            addr: None,
            kind: EventKind::ToolCalled,
            data: Payload::empty(),
            ig: false,
        })
        .collect();
    ledger.append_all(drafts).unwrap();

    let before = fs.bytes_read();
    let mut view = crate::HotView::new();
    let mut folded = 0u64;
    let mut ledger_bytes = 0u64;
    for line in ledger.read_raw_lines().unwrap() {
        ledger_bytes =
            ledger_bytes.saturating_add(u64::try_from(line.len().saturating_add(1)).unwrap());
        view.apply(&EventRecord::parse_line(&line).unwrap())
            .unwrap();
        folded = folded.saturating_add(1);
    }
    let read = fs.bytes_read().saturating_sub(before);
    eprintln!(
        "view_rebuild_read: {folded} records folded, {read} B read for a {ledger_bytes} B ledger"
    );
    assert_eq!(folded, 401, "every record is folded exactly once");
    assert_eq!(read, ledger_bytes, "a rebuild reads each byte once");
}
