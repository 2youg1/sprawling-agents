// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::fs;

use kernel::{EventDraft, EventKind, Payload, RunId, TimeMs};

use super::*;
use crate::jsonl::LineFault;

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

/// Four lines, one per segment, so the audit has to cross segments.
fn four_segment_ledger(dir: &Path) -> JsonlLedger {
    let (mut ledger, _) = JsonlLedger::open(dir, TimeMs::new(0)).unwrap();
    ledger.set_roll_bytes_for_test(1);
    for t in 0..4 {
        ledger.append_all(vec![draft(t)]).unwrap();
    }
    ledger
}

#[test]
fn an_intact_chain_is_whole_across_segments_and_a_torn_tail_is_not_a_line() {
    let tmp = tempfile::tempdir().unwrap();
    let _ledger = four_segment_ledger(tmp.path());
    let last = ledger_segments_at(tmp.path()).unwrap().pop().unwrap();
    let mut bytes = fs::read(&last).unwrap();
    bytes.extend_from_slice(b"{\"v\":1,\"torn");
    fs::write(&last, bytes).unwrap();

    assert_eq!(
        audit_chain(tmp.path()).unwrap(),
        ChainAudit::Whole { lines: 4 }
    );
}

#[test]
fn an_early_line_changed_in_place_breaks_the_chain_at_the_line_after_it() {
    let tmp = tempfile::tempdir().unwrap();
    let _ledger = four_segment_ledger(tmp.path());
    let first = ledger_segments_at(tmp.path()).unwrap().remove(0);
    let tampered = fs::read_to_string(&first)
        .unwrap()
        .replace("\"who\":\"city\"", "\"who\":\"town\"");
    fs::write(&first, tampered).unwrap();

    assert_eq!(
        audit_chain(tmp.path()).unwrap(),
        ChainAudit::Broken(LineFault::ChainBreak.into_ax(2))
    );
}

#[test]
fn a_tripped_halt_refuses_the_next_write_with_the_audits_own_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ledger = four_segment_ledger(tmp.path());
    let halt = ChainHalt::default();
    ledger.halt_on(halt.clone());
    let reason = LineFault::ChainBreak.into_ax(2);
    halt.trip(reason.clone());
    halt.trip(LineFault::ChainBreak.into_ax(3));

    let refused = ledger.append_all(vec![draft(9)]).unwrap_err().into_ax();

    assert_eq!((refused, halt.reason()), (reason.clone(), Some(&reason)));
    assert_eq!(ledger.read_raw_lines().unwrap().len(), 4);
}
