// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The read lengths of the one segment scan that builds an index: the
//! windowed read stops at a preallocated segment's zero tail (storage
//! D33), and a grown segment costs exactly its length.

use std::path::Path;

use kernel::{EventDraft, EventKind, Payload, RunId, Seq, TimeMs};

use super::{LedgerIndex, scan};
use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
use crate::jsonl::{FIRST_WINDOW_BYTES, JsonlLedger};
use crate::vfs::Vfs as _;

/// A full scan reads a preallocated segment the way a refresh does:
/// forward in windows, stopping at the window whose records end before
/// the window does (storage D32 and D33). The segment is the roll size
/// from the moment it is created, so a scan that lifted the whole file
/// would pay every zero between the last record and the roll size.
#[test]
fn a_full_scan_stops_at_the_zero_tail_of_a_preallocated_segment() {
    let roll: u64 = 1024 * 1024;
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: TornTail::None,
    });
    let dir = Path::new("ledger");
    let draft = |t: u64| EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(t),
        who: "city".to_owned(),
        addr: None,
        kind: EventKind::GateChecked,
        data: Payload::empty(),
        ig: false,
    };
    let (mut ledger, _) = JsonlLedger::open_preallocated(fs.clone(), dir, TimeMs::new(0)).unwrap();
    ledger.set_roll_bytes_for_test(roll);
    ledger.append_all((0..3).map(draft).collect()).unwrap();
    let segment = dir.join("ledger-00000000000000000000.jsonl");
    assert_eq!(
        fs.size(&segment).unwrap(),
        roll,
        "the fixture is preallocated"
    );

    let before = fs.bytes_read();
    let index = LedgerIndex {
        folded: scan(&fs, dir).unwrap(),
        vfs: std::sync::Mutex::new(Box::new(fs.clone())),
    };
    let read = fs.bytes_read().saturating_sub(before);
    assert_eq!(
        (read, index.len(), index.tail_seq()),
        (FIRST_WINDOW_BYTES, 3, Some(Seq::new(2))),
        "a scan reads the window that reaches the zero tail, not all {roll} bytes"
    );
}

/// A segment that was never preallocated rebuilds as it did: every byte
/// of it once, every record folded. The scan reads it in windows now,
/// so the byte count must come out as the segment's length rather than
/// as a whole number of windows.
#[test]
fn a_rebuild_of_a_grown_segment_reads_it_once_and_folds_every_record() {
    let fs = FaultFs::new(FaultPlan {
        cut_at_op: None,
        cut_on_write: None,
        torn_tail: TornTail::None,
    });
    let dir = Path::new("ledger");
    let (mut ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir, TimeMs::new(0)).unwrap();
    let drafts: Vec<EventDraft> = (0..400)
        .map(|t| EventDraft {
            run: RunId::from_bytes([5u8; 16]),
            t: TimeMs::new(t),
            who: "tester".to_owned(),
            addr: None,
            kind: EventKind::ToolCalled,
            data: Payload::empty(),
            ig: false,
        })
        .collect();
    ledger.append_all(drafts).unwrap();
    let segment = dir.join("ledger-00000000000000000000.jsonl");
    let size = fs.size(&segment).unwrap();
    assert!(
        size > FIRST_WINDOW_BYTES,
        "the fixture must span more than one window"
    );

    let before = fs.bytes_read();
    let index = LedgerIndex {
        folded: scan(&fs, dir).unwrap(),
        vfs: std::sync::Mutex::new(Box::new(fs.clone())),
    };
    let read = fs.bytes_read().saturating_sub(before);
    assert_eq!(
        (read, index.len(), index.tail_seq()),
        (size, 400, Some(Seq::new(399))),
        "a rebuild of a grown segment reads each byte once"
    );
}
