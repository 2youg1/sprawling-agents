// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The in-memory Ledger: second adapter of the kernel port. Bytes come from the same canonical producer as the durable
//! adapter, so simulation histories and real histories are comparable
//! byte for byte.
//!
//! Specified by `tools/citysim/spec/MemLedger.lean` §8-1.

use kernel::ledger::chain_hash;
use kernel::ledger::conformance::LedgerInspect;
use kernel::{AxError, B3Hash, EventDraft, EventRecord, EventRef, GENESIS_PREV, Ledger, Seq};

pub struct MemLedger {
    lines: Vec<Vec<u8>>,
    next_seq: Seq,
    prev: B3Hash,
}

impl MemLedger {
    pub fn new() -> Self {
        MemLedger {
            lines: Vec::new(),
            next_seq: Seq::FIRST,
            prev: GENESIS_PREV,
        }
    }

    /// The inherent read face (`tools/citysim/spec/MemLedger.lean` §8-1):
    /// the executor's report and byte comparisons read here; the
    /// `LedgerInspect` impl below stays the conformance suite's door.
    pub fn raw_lines(&self) -> &[Vec<u8>] {
        &self.lines
    }
}

impl Default for MemLedger {
    fn default() -> Self {
        MemLedger::new()
    }
}

impl Ledger for MemLedger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let record = EventRecord::from_draft(draft, self.next_seq, self.prev);
        let line = record.canonical_line()?;
        // Every fallible step comes before the first write, so a refused
        // draft leaves the ledger as it found it.
        self.next_seq = self.next_seq.next()?;
        self.prev = chain_hash(&line);
        self.lines.push(line);
        Ok(record.to_ref())
    }
}

impl LedgerInspect for MemLedger {
    fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError> {
        Ok(self.lines.clone())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;

    /// A ledger whose next seq is the last one a u64 holds has no seq to
    /// advance to, so it refuses the draft and is left as the refusal
    /// found it: no line, no seq and no chain digest moved.
    #[test]
    fn an_append_past_the_last_seq_leaves_the_ledger_unchanged() {
        let draft = EventDraft {
            run: kernel::RunId::CITY,
            t: kernel::TimeMs::new(1),
            who: "city".to_owned(),
            addr: None,
            kind: kernel::EventKind::CityInitialized,
            data: kernel::Payload::empty(),
            ig: false,
        };
        let mut ledger = MemLedger {
            lines: Vec::new(),
            next_seq: Seq::new(u64::MAX),
            prev: GENESIS_PREV,
        };
        let before = (ledger.lines.clone(), ledger.next_seq, ledger.prev);
        assert!(ledger.append(draft).is_err());
        assert_eq!((ledger.lines.clone(), ledger.next_seq, ledger.prev), before);
    }
}
