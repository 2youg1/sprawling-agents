// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The proof of the whole history a served city runs beside its writer
//! (sprawling-SPEC.md 8-90, 8-122). The halt it sets the verdict on is
//! attached by the worker (`chain_halt`); this file starts the thread
//! and says what it found.

use std::time::Instant;

use kernel::{AxCode, AxError, RunId};
use runtime::diagnostics::{Diagnostics, Level, Site};

use crate::serving::standing::monotonic_now;
use accounting::worker::chain_halt::ChainUnderAudit;
use accounting::worker::opening_cost::millis;

/// Proves the whole chain `watch` names on a thread of its own.
///
/// The thread holds the ledger directory, the halt, the records and
/// `log`, never the writer, so the writer never waits for the proof. A
/// whole chain lets the writer take lines; a broken one trips the halt,
/// and from then on every append is refused with the proof's own reason,
/// which goes to `log` too. `began` is when opening the city began, so
/// the moment commands are taken is read from the same point as the
/// first byte.
///
/// # Errors
/// `StorageFatal` when the thread cannot be started.
pub(super) fn audit_in_background(
    watch: ChainUnderAudit,
    log: Diagnostics,
    began: Instant,
) -> Result<std::thread::JoinHandle<()>, AxError> {
    std::thread::Builder::new()
        .name("sprawling-chain-audit".to_owned())
        .spawn(move || report_proof(watch, log, began))
        .map_err(|source| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the chain audit",
                source.to_string(),
            )
            .with_recovery("check process thread limits")
        })
}

/// Trips the halt it holds when dropped. The first verdict wins, so once
/// the proof has set one this does nothing; it matters only when the
/// thread unwinds without a verdict, so a writer waiting to close is
/// never left waiting.
struct Unsettled<'a>(&'a storage::ChainHalt);

impl Drop for Unsettled<'_> {
    fn drop(&mut self) {
        self.0.trip(
            AxError::failure(
                AxCode::StorageFatal,
                "prove the history",
                "the proof ended without a verdict",
            )
            .with_recovery("restart the server; the history is proved again when it opens"),
        );
    }
}

/// Runs the proof, sets the verdict, and says what it found. A ledger
/// that could not be read trips the halt too: a proof that did not finish
/// proved nothing whole, and the views and the standing resumed from
/// snapshots have only this proof reading the lines before them.
fn report_proof(watch: ChainUnderAudit, mut log: Diagnostics, began: Instant) {
    let ChainUnderAudit {
        halt,
        ledger_dir,
        at,
        records,
    } = watch;
    let started = monotonic_now();
    let unsettled = Unsettled(&halt);
    let said = match storage::prove_chain(&ledger_dir, &records) {
        Ok(storage::Proven {
            audit: storage::ChainAudit::Whole { lines },
            counted,
            unkept,
        }) => {
            halt.prove();
            let done = monotonic_now();
            let proved = format!(
                "the history is proved: {lines} lines, {} segment(s) by digest, {} lines checked, {} bytes read, {} bytes hashed, in {} ms, {} ms after opening began; commands are taken",
                counted.segments_by_digest,
                counted.lines_checked,
                counted.bytes_read,
                counted.bytes_hashed,
                millis(done.saturating_duration_since(started)),
                millis(done.saturating_duration_since(began)),
            );
            let unkept = unkept.map(|fault| {
                (
                    Level::Refuse,
                    format!(
                        "a record of the proof was not kept: {fault}; the next opening checks those segments line by line"
                    ),
                )
            });
            std::iter::once((Level::Effect, proved))
                .chain(unkept)
                .collect()
        }
        Ok(storage::Proven {
            audit: storage::ChainAudit::Broken(reason),
            ..
        }) => {
            let message = format!(
                "the ledger stopped taking writes: {reason}; {}",
                reason.recovery()
            );
            halt.trip(reason);
            vec![(Level::Refuse, message)]
        }
        Err(err) => {
            let err = err.into_ax();
            let message = format!(
                "the ledger stopped taking writes: the chain audit could not read it: {err}; {}",
                err.recovery()
            );
            halt.trip(err);
            vec![(Level::Refuse, message)]
        }
    };
    drop(unsettled);
    let site = Site {
        run: RunId::CITY,
        seq: at,
        module: "bin::assembly",
    };
    for (level, message) in said {
        log.write(level, site, &message);
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
mod tests;
