// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a worker inherits from a history it did not write.

use std::path::Path;

use kernel::AxError;
use runtime::replay::fold_ledger_dir;
use storage::{JsonlLedger, OpenReport};

// The governance fold lives in `views`, where the reading side keeps
// one too. Named here so the worker that judges from it reads under
// the same name a page is answered under.
pub(super) use accounting::views::Governance;
use accounting::views::Views;
use accounting::views::snapshot::start::{
    SnapshotFold, city_root_of, cut, cut_at, last_line, start_audited,
};

use super::Entrance;
use super::opening_cost::{OpeningCost, Phase};

mod collaboration;
mod session;
mod standing_start;
mod views_start;

pub(super) use collaboration::{Collaboration, INBOX_CAPACITY, new_inbox};
pub(super) use session::SessionOrigins;
use standing_start::StandingFolds;
pub(super) use standing_start::{as_json_text, from_json_text};
pub(crate) use views_start::start_served_views;

/// Everything a worker inherits from a history it did not write.
///
/// The governance half is `views::Governance`, the same type and the
/// same fold the served `Views` holds: the worker judges from it and a
/// page reads from it, and a rebuild makes the two equal.
pub(crate) struct Standing {
    pub(crate) book: gateway::EndpointBook,
    pub(super) governance: Governance,
    pub(super) collaboration: Collaboration,
    /// The keys of the commands this history already carried out. On
    /// the same pass as the other three: recognising a repeat across a
    /// restart must not cost a second read of the whole history.
    pub(super) entrance: Entrance,
    /// What each room's current session branched from, until the run
    /// that begins it is written.
    pub(super) origins: SessionOrigins,
    /// Whether the snapshot of these folds was cut. Not an error of the
    /// fold: a snapshot only shortens the next start.
    pub(super) cut: Result<(), AxError>,
}

impl Standing {
    /// The folds of one pass over the history, from the standing
    /// snapshot when one fits and from genesis otherwise, with a new
    /// snapshot cut at the last line folded (sprawling-SPEC 8-101).
    ///
    /// One pass for all six folds: recognising a repeat, an expiry or a
    /// session's origin across a restart must not cost a second read of
    /// the history, which `what_a_worker_holds_is_what_a_restart_rebuilds`
    /// holds. A snapshot that fails verification or does not decode is
    /// never trusted; the whole history is verified and folded instead.
    /// The whole chain is audited first, so a start from the snapshot
    /// never accepts a chain a whole fold would refuse: the snapshot's fit
    /// checks only the line at its seq, the ledger open scans only the last
    /// segment, and `fork` and `adopt` open a worker with no chain watch
    /// beside it.
    ///
    /// # Errors
    /// The audit's reason when the chain is broken or cannot be read,
    /// chain verification of what is folded, and whatever a fold says
    /// about a payload it cannot read; a cut that fails is in `cut`, not
    /// here.
    pub(crate) fn fold(ledger_dir: &Path) -> Result<Standing, AxError> {
        if !ledger_dir.exists() {
            return StandingFolds::empty(ledger_dir).settle(Ok(()));
        }
        let started = start_audited::<StandingFolds>(ledger_dir)?;
        let cut = cut(ledger_dir, &started);
        started.folded.settle(cut)
    }
}

/// What a served city starts from: its views and the worker's standing,
/// folded from one verified read of the history.
///
/// Each record is shown to the views and then to the standing, so the
/// first byte a person sees waits on one verified pass rather than two,
/// and the worker judges from exactly the history the pages answer from:
/// two reads at two moments could answer for two different histories.
///
/// The ledger is opened, and its writer lock taken, before the history is
/// read: the standing is what the worker decides from, so no line another
/// process appends may land between the fold and the lock. The opened
/// ledger travels with what opening it repaired and the standing it was
/// folded under.
///
/// A standing snapshot is cut at the last line folded, as every worker
/// open cuts one (sprawling-SPEC 8-101), so a worker opened over this city
/// later folds only what arrives after it; a cut that fails is in
/// `Standing.cut`, not here.
///
/// The ledger is opened at `now`, which the caller sampled: this module
/// reads no clock of its own (determinism rule 2).
///
/// Opening the ledger, the verifying pass with the folding inside it, and
/// the standing cut are each lapped on `cost` (sprawling-SPEC 8-121).
///
/// # Errors
/// Propagates opening the ledger, chain verification, and whatever a fold
/// says about a payload it cannot read.
pub(crate) fn fold_city(
    ledger_dir: &Path,
    now: kernel::TimeMs,
    cost: &mut OpeningCost,
) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError> {
    let (ledger, report) =
        JsonlLedger::open(ledger_dir, now).map_err(storage::StorageError::into_ax)?;
    cost.lap(Phase::OpenLedger);
    let mut views = Views::over(ledger_dir);
    let mut standing = StandingFolds::empty(city_root_of(ledger_dir));
    let index = fold_ledger_dir(ledger_dir, |record| {
        cost.folding(|| {
            views.apply(record)?;
            standing.absorb(record)
        })
    })?;
    cost.lap(Phase::VerifyAndFold { lines: index.len() });
    let cut =
        last_line(&index, ledger_dir).and_then(|last| cut_at(ledger_dir, &standing, last.as_ref()));
    cost.lap(Phase::CutStanding);
    views.hold_index(index, ledger_dir)?;
    Ok((views, (ledger, report, standing.settle(cut)?)))
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
