// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Instant;

use kernel::{AxCode, AxError, RunId};
use runtime::diagnostics::{Diagnostics, Level, Site};

use crate::serving::standing::monotonic_now;
use accounting::worker::chain_halt::ChainUnderAudit;
use accounting::worker::opening_cost::millis;

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
