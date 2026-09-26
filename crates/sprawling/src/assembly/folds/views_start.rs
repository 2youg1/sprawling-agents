// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the views start folding: after the snapshot, or from genesis
//! and why (sprawling-SPEC 8-91).

use std::path::Path;

use kernel::{AxError, EventRecord, RunId};
use runtime::diagnostics::{Diagnostics, Level, Site};

use crate::views::{Views, views_fold_version};

use super::snapshot_start::{SnapshotFold, Started, cut, start};

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
}

/// The views of the ledger in `ledger_dir`, from their snapshot when one
/// fits and from genesis otherwise.
///
/// # Errors
/// Those of [`start`].
pub(crate) fn start_views(ledger_dir: &Path) -> Result<Started<Views>, AxError> {
    start(ledger_dir)
}

/// The views `serve` starts with: [`start_views`], then a snapshot cut
/// at the last line it folded, then where the start began written to
/// `log` (sprawling-SPEC 8-91).
///
/// A cut that fails is a `Refuse` line in `log`, not an error: the
/// snapshot only shortens the next start, which folds from the older
/// snapshot or from genesis to the same views.
///
/// # Errors
/// Those of [`start_views`].
pub(crate) fn start_served_views(
    ledger_dir: &Path,
    log: &mut Diagnostics,
) -> Result<Views, AxError> {
    let started = start_views(ledger_dir)?;
    let site = Site {
        run: RunId::CITY,
        seq: started.last_seq(),
        module: "bin::assembly",
    };
    if let Err(fault) = cut(ledger_dir, &started) {
        log.write(
            Level::Refuse,
            site,
            &format!(
                "the views snapshot was not cut: {fault}; serving goes on, and the next start folds from the older snapshot or from genesis"
            ),
        );
    }
    log.write(Level::Effect, site, &format!("the views {}", started.from));
    Ok(started.folded)
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
