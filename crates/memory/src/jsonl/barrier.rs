// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The durability barrier's state: whether the ledger's in-memory
//! position still names what the disk holds (memory-SPEC 8-1).

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use kernel::{EventDraft, EventKind, EventRecord, Payload, RunId, TimeMs};

    use crate::fault_fs::{FaultFs, FaultPlan, TornTail};
    use crate::jsonl::JsonlLedger;

    fn draft(t: u64) -> EventDraft {
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(t),
            who: "city".to_string(),
            addr: None,
            kind: EventKind::RunStarted,
            data: Payload::empty(),
            ig: false,
        }
    }

    /// A wave whose write dies partway leaves torn bytes at the tail. A
    /// ledger that keeps appending after them buries a later wave behind
    /// the tear, and the next open truncates that wave away although its
    /// `append_all` answered `Ok`. Whatever the ledger answers after a
    /// failed barrier, no `Ok` may name a record the reopened ledger
    /// does not hold.
    #[test]
    fn no_append_after_a_failed_barrier_claims_a_record_the_disk_loses() {
        let fs = FaultFs::new(FaultPlan {
            cut_at_op: None,
            cut_on_write: Some("424242"),
            torn_tail: TornTail::KeepBytes(10),
        });
        let dir = Path::new("l");
        let (mut ledger, _) = JsonlLedger::open_faulty(fs.clone(), dir, TimeMs::new(0)).unwrap();
        let mut claimed = BTreeSet::new();
        for t in [1, 424_242, 3] {
            if let Ok(refs) = ledger.append_all(vec![draft(t)]) {
                claimed.extend(refs.iter().map(|r| (r.seq(), t)));
            }
        }
        drop(ledger);

        let (reopened, _) = JsonlLedger::open_faulty(fs, dir, TimeMs::new(9)).unwrap();
        let held: BTreeSet<_> = reopened
            .read_raw_lines()
            .unwrap()
            .iter()
            .map(|line| EventRecord::parse_line(line).unwrap())
            .map(|record| (record.seq(), record.t().value()))
            .collect();
        let lost: Vec<_> = claimed.difference(&held).collect();
        assert!(lost.is_empty(), "claimed but lost on reopen: {lost:?}");
    }
}
