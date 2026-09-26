// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

use kernel::ledger::chain_hash;
use kernel::{EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs};

use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
use crate::jsonl::JsonlLedger;

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
