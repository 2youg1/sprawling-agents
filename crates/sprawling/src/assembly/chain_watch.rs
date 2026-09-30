// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The whole-chain audit a served city runs beside its writer
//! (sprawling-SPEC.md 8-90). The halt it trips is attached by the worker
//! (`chain_halt`); this file starts the thread and says what it found.

use std::path::Path;

use kernel::{AxCode, AxError, RunId, Seq};
use runtime::diagnostics::{Diagnostics, Level, Site};

use super::chain_halt::ChainUnderAudit;
use super::opening_cost::millis;
use crate::serving::standing::monotonic_now;

/// Walks the whole chain `watch` names on a thread of its own.
///
/// The thread holds the ledger directory, the halt and `log`, never the
/// writer, so the writer never waits for the audit. A broken chain trips
/// the halt, and from then on every append is refused with the audit's
/// own reason; the same reason goes to `log`.
///
/// # Errors
/// `StorageFatal` when the thread cannot be started.
pub(super) fn audit_in_background(
    watch: ChainUnderAudit,
    log: Diagnostics,
) -> Result<std::thread::JoinHandle<()>, AxError> {
    let ChainUnderAudit {
        halt,
        ledger_dir,
        at,
    } = watch;
    std::thread::Builder::new()
        .name("sprawling-chain-audit".to_owned())
        .spawn(move || report_audit(&ledger_dir, &halt, at, log))
        .map_err(|source| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the chain audit",
                source.to_string(),
            )
            .with_recovery("check process thread limits")
        })
}

/// Runs the audit, trips `halt` unless it proved the whole chain, and
/// says what it found. A ledger that could not be read trips it too: an
/// audit that did not finish proved nothing whole, and a view resumed
/// from a snapshot has only this audit reading the lines before it. A
/// whole chain is reported with how long its pass took, read on the
/// monotonic sampling point (sprawling-SPEC.md 8-90).
fn report_audit(dir: &Path, halt: &storage::ChainHalt, at: Seq, mut log: Diagnostics) {
    let began = monotonic_now();
    let (level, message) = match storage::audit_chain(dir) {
        Ok(storage::ChainAudit::Whole { lines }) => (
            Level::Effect,
            format!(
                "the whole ledger chain verified: {lines} lines in {} ms",
                millis(monotonic_now().saturating_duration_since(began))
            ),
        ),
        Ok(storage::ChainAudit::Broken(reason)) => {
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
