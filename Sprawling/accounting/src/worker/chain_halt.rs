// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::Seq;
use kernel::layout::CityLayout;

use super::RunWorker;
use crate::views::snapshot::start::proof_dir;

pub struct ChainUnderAudit {
    pub halt: storage::ChainHalt,
    pub ledger_dir: PathBuf,
    pub at: Seq,
    pub records: storage::ProofRecords,
}

impl RunWorker {
    pub fn chain_under_audit(&mut self, halt: storage::ChainHalt) -> ChainUnderAudit {
        self.ledger.halt_on(halt.clone());
        ChainUnderAudit {
            halt,
            ledger_dir: CityLayout::new(&self.city_root).ledger(),
            at: self.ledger.position(),
            records: self.ledger.proof_records(&proof_dir(&self.city_root)),
        }
    }

    pub fn await_proof(&self) {
        self.ledger.await_verdict();
    }
}
