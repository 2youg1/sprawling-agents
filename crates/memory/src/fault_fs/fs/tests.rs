// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::*;
use crate::jsonl::JsonlLedger;
use crate::vfs::Vfs;
use kernel::{
    EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs, chain_hash,
};
use std::path::{Path, PathBuf};
fn plain() -> FaultPlan {
    FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: TornTail::None,
    }
}
fn cut_at(op: u64, torn: TornTail) -> FaultPlan {
    FaultPlan {
        cut_at_op: Some(op),
        cut_on_write: None,
        torn_tail: torn,
    }
}

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
fn waves() -> Vec<Vec<EventDraft>> {
    vec![
        vec![
            draft(EventKind::CityInitialized, 1),
            draft(EventKind::BuildingCreated, 2),
        ],
        vec![draft(EventKind::RunStarted, 3)],
        vec![
            draft(EventKind::GateChecked, 4),
            draft(EventKind::RunFrozen, 5),
        ],
    ]
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

/// A3 point 1 (power loss at EventRecord append), whole-matrix form:
/// cut at every single vfs op; after reopen the chain verifies and
/// every wave that returned Ok is still there, byte-exact.
#[test]
fn power_cut_matrix_over_every_op_keeps_acknowledged_waves() {
    let dir = Path::new("city/ledger");

    // Baseline: full run, no cut. Also measures total op count.
    let fs = FaultFs::new(plain());
    let (mut ledger, _) =
        JsonlLedger::open_with(Box::new(fs.clone()), dir, TimeMs::new(0)).expect("clean open");
    let mut cumulative: Vec<usize> = Vec::new();
    for wave in waves() {
        ledger.append_all(wave).expect("clean append");
        cumulative.push(ledger.read_raw_lines().unwrap().len());
    }
    let baseline = ledger.read_raw_lines().unwrap();
    let total_ops = fs.op_count();
    drop(ledger);

    for torn in [TornTail::None, TornTail::KeepBytes(7)] {
        for cut in 1..=total_ops {
            let fs = FaultFs::new(cut_at(cut, torn));
            let mut acknowledged = 0usize;
            // Err from open = power died during it; nothing ran.
            if let Ok((mut ledger, _)) =
                JsonlLedger::open_with(Box::new(fs.clone()), dir, TimeMs::new(0))
            {
                for wave in waves() {
                    match ledger.append_all(wave) {
                        Ok(_) => {
                            acknowledged = ledger
                                .read_raw_lines()
                                .map(|l| l.len())
                                .unwrap_or(acknowledged)
                        }
                        Err(_) => break, // crash-only: the process dies here
                    }
                }
            }

            // When the cut lands during the reopen itself (a second
            // power loss, possibly mid-recovery), the next open must
            // succeed — recovery is idempotent and the plan is spent.
            let reopened = match JsonlLedger::open_with(Box::new(fs.clone()), dir, TimeMs::new(99))
            {
                Ok((ledger, _)) => ledger,
                Err(_) => {
                    let (ledger, _) =
                        JsonlLedger::open_with(Box::new(fs.clone()), dir, TimeMs::new(99))
                            .unwrap_or_else(|e| {
                                panic!("second reopen after cut {cut} ({torn:?}): {e}")
                            });
                    ledger
                }
            };
            let lines = reopened.read_raw_lines().unwrap();
            verify_chain(&lines);
            let survivors: Vec<&Vec<u8>> = lines
                .iter()
                .filter(|l| EventRecord::parse_line(l).unwrap().kind() != EventKind::LogTruncated)
                .collect();
            assert!(
                survivors.len() >= acknowledged,
                "cut {cut} ({torn:?}): acknowledged waves must survive"
            );
            for (mine, original) in survivors.iter().zip(baseline.iter()) {
                assert_eq!(
                    *mine, original,
                    "cut {cut} ({torn:?}): survivors must be a byte-exact prefix"
                );
            }
        }
    }
}
#[test]
fn a_named_write_is_the_one_that_dies_and_only_once() {
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: Some("needle"),
        torn_tail: TornTail::None,
    });
    let mut v: Box<dyn Vfs> = Box::new(fs.clone());
    let dir = PathBuf::from("d");
    let file = dir.join("f");
    v.create_dir_all(&dir).unwrap();
    v.append(&file, b"safe").unwrap();
    v.sync_data(&file).unwrap();
    v.sync_dir(&dir).unwrap();
    // Ops before it are untouched, whatever their number.
    assert!(v.append(&file, b"carrying a needle here").is_err());
    // Spent: the same needle rides the next write and it lands.
    v.append(&file, b"needle again").unwrap();
    v.sync_data(&file).unwrap();
    assert_eq!(v.read(&file).unwrap(), b"safeneedle again");
}

#[test]
fn unsynced_bytes_vanish_and_synced_bytes_survive() {
    let fs = FaultFs::new(plain());
    let mut v: Box<dyn Vfs> = Box::new(fs.clone());
    let dir = PathBuf::from("d");
    let file = dir.join("f");
    v.create_dir_all(&dir).unwrap();
    v.append(&file, b"durable").unwrap();
    v.sync_data(&file).unwrap();
    v.sync_dir(&dir).unwrap();
    v.append(&file, b"+lost").unwrap();
    fs.power_cut();
    assert_eq!(v.read(&file).unwrap(), b"durable");
}

#[test]
fn torn_tail_keeps_a_prefix_of_the_unsynced_delta() {
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: TornTail::KeepBytes(3),
    });
    let mut v: Box<dyn Vfs> = Box::new(fs.clone());
    let dir = PathBuf::from("d");
    let file = dir.join("f");
    v.create_dir_all(&dir).unwrap();
    v.append(&file, b"ok").unwrap();
    v.sync_data(&file).unwrap();
    v.sync_dir(&dir).unwrap();
    v.append(&file, b"abcdef").unwrap();
    fs.power_cut();
    assert_eq!(v.read(&file).unwrap(), b"okabc");
}

#[test]
fn a_file_without_a_synced_dir_entry_vanishes_entirely() {
    let fs = FaultFs::new(plain());
    let mut v: Box<dyn Vfs> = Box::new(fs.clone());
    let dir = PathBuf::from("d");
    let file = dir.join("f");
    v.create_dir_all(&dir).unwrap();
    v.append(&file, b"synced but entry is not").unwrap();
    v.sync_data(&file).unwrap();
    // no sync_dir: stricter than POSIX on purpose (memory-SPEC 8-2).
    fs.power_cut();
    assert!(!v.exists(&file));
    assert!(v.read(&file).is_err());
}

#[test]
fn the_cut_op_fails_and_later_ops_proceed() {
    let fs = FaultFs::new(cut_at(3, TornTail::None));
    let mut v: Box<dyn Vfs> = Box::new(fs.clone());
    let dir = PathBuf::from("d");
    v.create_dir_all(&dir).unwrap(); // op 1
    v.append(&dir.join("f"), b"x").unwrap(); // op 2
    let denied = v.sync_data(&dir.join("f")); // op 3: power dies here
    assert!(denied.is_err());
    assert!(v.list(&dir).is_ok(), "after the cut the plan is consumed");
    assert_eq!(fs.op_count(), 4);
}
