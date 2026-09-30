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

use crate::views::Views;
use crate::views::snapshot::start::cut;
use crate::worker::opening_cost::{OpeningCost, Phase};

use super::{Standing, fold_city};

/// What `serve` starts from: [`fold_city`] over the ledger opened at
/// `now`, then a views snapshot cut at the last line the views folded, so
/// the next start and a one-shot read afterwards fold only what arrives
/// after it (sprawling-SPEC 8-91); nothing is cut when the views resumed
/// from their snapshot and folded nothing past it.
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
pub fn start_served_views(
    ledger_dir: &Path,
    now: kernel::TimeMs,
    log: &mut Diagnostics,
    cost: &mut OpeningCost,
) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError> {
    let (views, held) = fold_city(ledger_dir, now, cost, log)?;
    let cut = cut(ledger_dir, &views);
    cost.lap(Phase::CutViews);
    if let Err(fault) = cut {
        log.write(
            Level::Refuse,
            Site {
                run: RunId::CITY,
                seq: views.last_seq().unwrap_or(Seq::FIRST),
                module: "accounting::worker",
            },
            &format!(
                "the views snapshot was not cut: {fault}; serving goes on, and the next read folds from the older snapshot or from genesis"
            ),
        );
    }
    Ok((views.folded, held))
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
