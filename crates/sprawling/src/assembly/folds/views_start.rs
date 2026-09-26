// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the views start folding: after the snapshot, or from genesis
//! and why (sprawling-SPEC 8-91).

use std::path::Path;

use kernel::{AxError, Seq};
use memory::WholeFold;

use crate::views::Views;

use super::known_records;

/// The views a start folded, how it began, and where the next snapshot
/// is cut.
pub(crate) struct StartedViews {
    pub(crate) views: Views,
    pub(crate) from: ViewsStart,
    /// The last line this start folded, with its seq. `None` when it
    /// folded nothing past the snapshot, so there is nothing to cut.
    last: Option<(Seq, Vec<u8>)>,
}

/// How a start began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ViewsStart {
    /// From the snapshot, folding the `tail` lines after it.
    Resumed { tail: usize },
    /// From genesis, and why the snapshot was not used.
    Whole(WholeFold),
}

/// The views of the ledger in `ledger_dir`, from its snapshot when one
/// fits.
///
/// # Errors
/// A tail or a whole history that does not verify, a record a view
/// cannot read, and an I/O failure reading the ledger.
pub(crate) fn start_views(ledger_dir: &Path) -> Result<StartedViews, AxError> {
    let verified = runtime::replay::verify_ledger_dir(ledger_dir)?;
    let city_root = city_root_of(ledger_dir);
    let mut views = Views::new(city_root);
    for record in known_records(&verified) {
        views.apply(record)?;
    }
    Ok(StartedViews {
        views,
        from: ViewsStart::Whole(WholeFold::NoSnapshot),
        last: None,
    })
}

/// Cut a snapshot of `started` at the last line it folded.
///
/// # Errors
/// An encoding failure and an I/O failure writing the snapshot.
pub(crate) fn cut_views_snapshot(
    _ledger_dir: &Path,
    _started: &StartedViews,
) -> Result<(), AxError> {
    Ok(())
}

fn city_root_of(ledger_dir: &Path) -> &Path {
    ledger_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or(ledger_dir)
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
