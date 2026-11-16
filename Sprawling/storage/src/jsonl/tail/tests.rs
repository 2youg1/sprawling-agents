// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
use crate::jsonl::JsonlLedger;
use kernel::{EventDraft, EventKind, Payload, RunId, TimeMs};

fn city_of(records: u64, roll_bytes: u64) -> (tempfile::TempDir, FaultFs, Vec<Vec<u8>>) {
    let dir = tempfile::tempdir().unwrap();
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: TornTail::None,
    });
    let (mut ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir.path(), TimeMs::new(0)).unwrap();
    ledger.roll_bytes = roll_bytes;
    for t in 0..records {
        ledger
            .append_all(vec![EventDraft {
                run: RunId::from_bytes([5u8; 16]),
                t: TimeMs::new(t),
                who: "tester".to_owned(),
                addr: None,
                kind: EventKind::ToolCalled,
                data: Payload::empty(),
                ig: false,
            }])
            .unwrap();
    }
    let lines = ledger.read_raw_lines().unwrap();
    (dir, fs, lines)
}

fn segments(fs: &FaultFs, dir: &Path) -> Vec<PathBuf> {
    fs.list(dir)
        .unwrap()
        .into_iter()
        .filter(|p| is_segment(p))
        .collect()
}

#[test]
fn the_tail_yields_the_newest_lines_first_and_reads_only_their_bytes() {
    let (dir, fs, lines) = city_of(400, 4096);
    let mut torn = fs.clone();
    let last = segments(&fs, dir.path()).pop().unwrap();
    torn.append(&last, b"{\"v\":1,\"seq\":4").unwrap();
    let total: u64 = lines.iter().map(|l| l.len() as u64 + 1).sum();

    let before = fs.bytes_read();
    let newest: Vec<Vec<u8>> = TailLines::through(Box::new(fs.clone()), dir.path())
        .unwrap()
        .take(5)
        .map(|line| line.unwrap().raw)
        .collect();
    let read = fs.bytes_read() - before;

    let expected: Vec<Vec<u8>> = lines.iter().rev().take(5).cloned().collect();
    assert_eq!(newest, expected);
    assert!(
        read < total / 8,
        "read {read} of {total} bytes for five lines"
    );

    let every: Vec<Vec<u8>> = TailLines::through(Box::new(fs.clone()), dir.path())
        .unwrap()
        .map(|line| line.unwrap().raw)
        .collect();
    let reversed: Vec<Vec<u8>> = lines.iter().rev().cloned().collect();
    assert_eq!(every, reversed);
}

#[test]
fn a_line_that_does_not_link_to_the_newer_one_ends_the_walk_with_its_line() {
    let (dir, fs, lines) = city_of(40, 1 << 20);
    let mut cut = fs.clone();
    let last = segments(&fs, dir.path()).pop().unwrap();
    let bytes = fs.read(&last).unwrap();
    let kept: Vec<&[u8]> = bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    let missing = kept.len() - 3;
    let mut rewritten = Vec::new();
    for (index, line) in kept.iter().enumerate() {
        if index != missing {
            rewritten.extend_from_slice(line);
            rewritten.push(b'\n');
        }
    }
    cut.truncate(&last, 0).unwrap();
    cut.append(&last, &rewritten).unwrap();

    let walked: Vec<Result<TailLine, StorageError>> =
        TailLines::through(Box::new(fs.clone()), dir.path())
            .unwrap()
            .collect();
    let newer: Vec<Vec<u8>> = walked
        .iter()
        .take(2)
        .map(|line| line.as_ref().unwrap().raw.clone())
        .collect();
    assert_eq!(
        newer,
        lines.iter().rev().take(2).cloned().collect::<Vec<_>>()
    );
    assert_eq!(
        walked.len(),
        3,
        "the walk stops at the first line that does not link"
    );
    let Some(Err(StorageError::Envelope { line, .. })) = walked.last() else {
        panic!("the third line back must be refused");
    };
    assert_eq!(*line, lines.len() as u64 - 2);
}

/// A blank line is no line: the walk steps over it, as the forward
/// reader of raw lines does.
#[test]
fn a_blank_line_is_stepped_over() {
    let (dir, fs, lines) = city_of(2, 1 << 20);
    let last = segments(&fs, dir.path()).pop().unwrap();
    let bytes = fs.read(&last).unwrap();
    let mut blank = fs.clone();
    blank.truncate(&last, 0).unwrap();
    let first_end = bytes.iter().position(|byte| *byte == b'\n').unwrap() + 1;
    blank
        .append(
            &last,
            &[&bytes[..first_end], b"\n", &bytes[first_end..]].concat(),
        )
        .unwrap();

    let walked: Vec<Vec<u8>> = TailLines::through(Box::new(fs.clone()), dir.path())
        .unwrap()
        .map(|line| line.unwrap().raw)
        .collect();
    assert_eq!(walked, lines.iter().rev().cloned().collect::<Vec<_>>());
}

/// A line longer than the first window comes back whole.
#[test]
fn a_line_longer_than_the_window_comes_back_whole() {
    let dir = tempfile::tempdir().unwrap();
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: TornTail::None,
    });
    let (mut ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir.path(), TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![EventDraft {
            run: RunId::from_bytes([5u8; 16]),
            t: TimeMs::new(0),
            who: "x".repeat(3 * 4096),
            addr: None,
            kind: EventKind::ToolCalled,
            data: Payload::empty(),
            ig: false,
        }])
        .unwrap();
    let lines = ledger.read_raw_lines().unwrap();

    let walked: Vec<Vec<u8>> = TailLines::through(Box::new(fs.clone()), dir.path())
        .unwrap()
        .map(|line| line.unwrap().raw)
        .collect();
    assert_eq!(walked, lines);
}
