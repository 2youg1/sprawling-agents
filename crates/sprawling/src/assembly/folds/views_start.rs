// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the views start folding: after the snapshot, or from genesis
//! and why; and the snapshot a served city cuts (sprawling-SPEC 8-91).

use std::path::Path;

use kernel::{AxError, EventRecord, RunId, Seq};
use memory::{JsonlLedger, LedgerIndex, OpenReport};
use runtime::diagnostics::{Diagnostics, Level, Site};

use crate::views::{Views, views_fold_version};

use super::snapshot_start::{SnapshotFold, cut_at};
use super::{Standing, epoch_of, fold_city};

impl SnapshotFold for Views {
    const DIR: &'static str = "views";

    fn fold_version() -> u32 {
        views_fold_version()
    }

    fn empty(city_root: &Path) -> Views {
        Views::new(city_root)
    }

    fn decode(city_root: &Path, bytes: &[u8]) -> Result<Views, AxError> {
        Views::decode(city_root, bytes)
    }

    fn encode(&self) -> Result<Vec<u8>, AxError> {
        Views::encode(self)
    }

    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.apply(record)
    }

    /// The views answer later questions through the index, and name the
    /// history by the epoch read from its genesis line. After a resume
    /// the index starts empty and is refreshed by the first question
    /// that reads the ledger.
    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError> {
        self.adopt_epoch(epoch_of(&index, ledger_dir)?);
        self.hold_index(index);
        Ok(())
    }
}

/// What `serve` starts from: [`fold_city`], then a views snapshot cut at
/// the last line it folded, so a one-shot read afterwards folds only what
/// arrives after it (sprawling-SPEC 8-91).
///
/// `serve` itself folds the whole history rather than resuming: the
/// worker's standing is folded on the same pass, and the two snapshots
/// are cut at different moments, so starting the views from their
/// snapshot would add a second read rather than remove one.
///
/// A cut that fails is a `Refuse` line in `log`, not an error: the
/// snapshot only shortens a later read, which folds from the older
/// snapshot or from genesis to the same views.
///
/// # Errors
/// Those of [`fold_city`].
pub(crate) fn start_served_views(
    ledger_dir: &Path,
    log: &mut Diagnostics,
) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError> {
    let (views, held) = fold_city(ledger_dir)?;
    let last = views.last_folded_line(ledger_dir);
    let site = Site {
        run: RunId::CITY,
        seq: last
            .as_ref()
            .ok()
            .and_then(|last| last.as_ref().map(|(seq, _)| *seq))
            .unwrap_or(Seq::FIRST),
        module: "bin::assembly",
    };
    let cut = last.and_then(|last| cut_at(ledger_dir, &views, last.as_ref()));
    if let Err(fault) = cut {
        log.write(
            Level::Refuse,
            site,
            &format!(
                "the views snapshot was not cut: {fault}; serving goes on, and the next read folds from the older snapshot or from genesis"
            ),
        );
    }
    Ok((views, held))
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
