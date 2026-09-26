// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole-chain audit a served city runs beside its writer, and the
//! halt a broken chain trips on that writer (sprawling-SPEC.md 8-90).

use std::path::Path;

use kernel::{AxCode, AxError, RunId, Seq};
use runtime::diagnostics::{Diagnostics, Level, Site};

use super::RunWorker;

impl RunWorker {
    /// Attaches a fresh halt to this worker's writer, then walks the
    /// whole chain on a thread of its own.
    ///
    /// The thread holds the ledger directory, the halt and `log`, never
    /// the writer, so the writer never waits for the audit. A broken
    /// chain trips the halt, and from then on every append is refused
    /// with the audit's own reason; the same reason goes to `log`.
    ///
    /// # Errors
    /// `StorageFatal` when the thread cannot be started.
    pub(crate) fn audit_chain_in_background(
        &mut self,
        log: Diagnostics,
    ) -> Result<std::thread::JoinHandle<()>, AxError> {
        let halt = memory::ChainHalt::default();
        self.ledger.halt_on(halt.clone());
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        let at = self.ledger.position();
        std::thread::Builder::new()
            .name("sprawling-chain-audit".to_owned())
            .spawn(move || report_audit(&dir, &halt, at, log))
            .map_err(|source| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "start the chain audit",
                    source.to_string(),
                )
                .with_recovery("check process thread limits")
            })
    }
}

/// Runs the audit, trips `halt` unless it proved the whole chain, and
/// says what it found. A ledger that could not be read trips it too: an
/// audit that did not finish proved nothing whole, and a view resumed
/// from a snapshot has only this audit reading the lines before it.
fn report_audit(dir: &Path, halt: &memory::ChainHalt, at: Seq, mut log: Diagnostics) {
    let (level, message) = match memory::audit_chain(dir) {
        Ok(memory::ChainAudit::Whole { lines }) => (
            Level::Effect,
            format!("the whole ledger chain verified: {lines} lines"),
        ),
        Ok(memory::ChainAudit::Broken(reason)) => {
            let message = format!(
                "the ledger stopped taking writes: {reason}; {}",
                reason.recovery()
            );
            halt.trip(reason);
            (Level::Refuse, message)
        }
        Err(err) => {
            let err = err.into_ax();
            let message = format!(
                "the ledger stopped taking writes: the chain audit could not read it: {err}; {}",
                err.recovery()
            );
            halt.trip(err);
            (Level::Refuse, message)
        }
    };
    let site = Site {
        run: RunId::CITY,
        seq: at,
        module: "bin::assembly",
    };
    log.write(level, site, &message);
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
