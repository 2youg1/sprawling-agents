// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::Seq;

use crate::error::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Barrier {
    Whole,
    Broken,
}

impl Barrier {

    pub(crate) fn admit(&self, dir: &Path, at: Seq) -> Result<(), StorageError> {
        match self {
            Barrier::Whole => Ok(()),
            Barrier::Broken => Err(StorageError::LedgerBroken {
                dir: dir.to_path_buf(),
                at,
            }),
        }
    }
}

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
