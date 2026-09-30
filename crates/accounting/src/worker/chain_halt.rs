// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The halt a background chain audit trips, attached to this worker's
//! writer (sprawling-SPEC.md 8-90).
//!
//! The audit itself runs on a thread the assembly root starts
//! (`bin::assembly::chain_watch`); what it needs from the worker is
//! this value, so the thread never holds the writer.

use std::path::PathBuf;

use kernel::Seq;

use super::RunWorker;

/// What an audit of this city's chain needs, and nothing of the writer:
/// the halt now attached to it, where the ledger is, and the position
/// the audit starts from.
pub(crate) struct ChainUnderAudit {
    pub(crate) halt: storage::ChainHalt,
    pub(crate) ledger_dir: PathBuf,
    pub(crate) at: Seq,
}

impl RunWorker {
    /// Attaches a fresh halt to this worker's writer and hands back what
    /// an audit of the chain needs.
    ///
    /// From the moment the audit trips the halt, every append is refused
    /// with the audit's own reason.
    pub(crate) fn chain_under_audit(&mut self) -> ChainUnderAudit {
        let halt = storage::ChainHalt::default();
        self.ledger.halt_on(halt.clone());
        ChainUnderAudit {
            halt,
            ledger_dir: kernel::layout::CityLayout::new(&self.city_root).ledger(),
            at: self.ledger.position(),
        }
    }
}
