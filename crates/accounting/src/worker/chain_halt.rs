// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verdict a background proof of the history sets, attached to this
//! worker's writer (sprawling-SPEC.md 8-90, 8-122).
//!
//! The proof itself runs on a thread the assembly root starts
//! (`bin::assembly::chain_watch`); what it needs from the worker is
//! this value, so the thread never holds the writer.

use std::path::PathBuf;

use kernel::Seq;
use kernel::layout::CityLayout;

use super::RunWorker;
use crate::views::snapshot::start::proof_dir;

/// What a proof of this city's chain needs, and nothing of the writer:
/// the halt now attached to it, where the ledger is, the position the
/// proof starts from, and the records it reads and may write.
pub struct ChainUnderAudit {
    pub halt: storage::ChainHalt,
    pub ledger_dir: PathBuf,
    pub at: Seq,
    pub records: storage::ProofRecords,
}

impl RunWorker {
    /// Attaches `halt`, which awaits the proof, to this worker's writer
    /// and hands back what a proof of the chain needs. The halt is made
    /// by the caller, so the views a served city answers from can watch
    /// the same verdict (sprawling-SPEC.md 8-134).
    ///
    /// From this moment until the proof sets its verdict, every append
    /// is refused with `E_HISTORY_UNPROVEN`; after a broken verdict,
    /// with the proof's own reason. The records are writable because
    /// this worker's ledger holds the writer lock.
    pub fn chain_under_audit(&mut self, halt: storage::ChainHalt) -> ChainUnderAudit {
        self.ledger.halt_on(halt.clone());
        ChainUnderAudit {
            halt,
            ledger_dir: CityLayout::new(&self.city_root).ledger(),
            at: self.ledger.position(),
            records: self.ledger.proof_records(&proof_dir(&self.city_root)),
        }
    }

    /// Waits until the proof this worker's writer awaits has a verdict;
    /// at once for a writer that awaits none. A city closing before its
    /// proof finishes writes its handoff after the verdict, rather than
    /// having it refused (sprawling-SPEC.md 8-90).
    pub fn await_proof(&self) {
        self.ledger.await_verdict();
    }
}
