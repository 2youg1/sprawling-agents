// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The snapshot a served city cuts of its views (sprawling-SPEC 8-91);
//! where the views start folding is `views::snapshot::start`.

use std::path::Path;

use kernel::{AxError, RunId, Seq};
use runtime::diagnostics::{Diagnostics, Level, Site};
use storage::{JsonlLedger, OpenReport};

use crate::assembly::opening_cost::{OpeningCost, Phase};
use accounting::views::Views;
use accounting::views::snapshot::start::cut_at;

use super::{Standing, fold_city};

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
/// The phases of [`fold_city`] and the views cut are lapped on `cost`
/// (sprawling-SPEC 8-121).
///
/// # Errors
/// Those of [`fold_city`].
pub(crate) fn start_served_views(
    ledger_dir: &Path,
    log: &mut Diagnostics,
    cost: &mut OpeningCost,
) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError> {
    let (views, held) = fold_city(ledger_dir, cost)?;
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
    cost.lap(Phase::CutViews);
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
