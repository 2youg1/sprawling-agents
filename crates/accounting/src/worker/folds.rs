// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a worker inherits from a history it did not write.

use std::path::Path;

use kernel::{AxError, RunId, Seq};
use runtime::diagnostics::{Diagnostics, Level, Site};
use storage::{JsonlLedger, OpenReport};

// The governance fold lives in `views`, where the reading side keeps
// one too. Named here so the worker that judges from it reads under
// the same name a page is answered under.
pub(super) use crate::views::Governance;
use crate::views::Views;
use crate::views::snapshot::start::{
    SnapshotFold, Started, city_root_of, cut, proof_dir, start_audited, start_both,
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
pub use views_start::start_served_views;

/// Everything a worker inherits from a history it did not write.
///
/// The governance half is `views::Governance`, the same type and the
/// same fold the served `Views` holds: the worker judges from it and a
/// page reads from it, and a rebuild makes the two equal.
pub struct Standing {
    pub book: gateway::EndpointBook,
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
    pub fn fold(ledger_dir: &Path) -> Result<Standing, AxError> {
        if !ledger_dir.exists() {
            return StandingFolds::empty(ledger_dir).settle(Ok(()));
        }
        let started = start_audited::<StandingFolds>(ledger_dir)?.started;
        let cut = cut(ledger_dir, &started);
        started.folded.settle(cut)
    }
}

/// The opened ledger, what opening it repaired, and the standing folded
/// under its writer lock: what a served worker is handed to hold.
pub(crate) type Held = (JsonlLedger, OpenReport, Standing);

/// What a served city starts from: its views and the worker's standing,
/// started on one pass from the earlier of their two snapshots, or from
/// genesis when either cannot resume (sprawling-SPEC 8-122).
///
/// Only the lines the snapshots have not seen are checked before the
/// first byte; the history before them is proved behind it, and the
/// writer takes no line until that proof is whole (8-90). The views and
/// the standing are started from the same pass, so the worker judges
/// from exactly the history the pages answer from.
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
/// `Standing.cut`, not here. Where each fold started, and why, is one
/// `Effect` line in `log`.
///
/// The ledger is opened at `now`, which the caller sampled: this module
/// reads no clock of its own (determinism rule 2).
///
/// The ledger's last segment is proved by the record the last proof of
/// this history wrote, when it holds, rather than line by line
/// (sprawling-SPEC 8-144).
///
/// Opening the ledger, the pass, and the standing cut are each lapped on
/// `cost` (sprawling-SPEC 8-121).
///
/// # Errors
/// Propagates opening the ledger, chain verification of the lines folded,
/// and whatever a fold says about a payload it cannot read.
pub(crate) fn fold_city(
    ledger_dir: &Path,
    now: kernel::TimeMs,
    cost: &mut OpeningCost,
    log: &mut Diagnostics,
) -> Result<(Started<Views>, Held), AxError> {
    // The last proof's records are read, never written, here: the proof
    // that holds the lock writes them (`crates/storage/Spec.lean` §8-34).
    let records = storage::ProofRecords::read_only(&proof_dir(city_root_of(ledger_dir)));
    let (ledger, report) = JsonlLedger::open_reusing(ledger_dir, now, &records)
        .map_err(storage::StorageError::into_ax)?;
    cost.lap(Phase::OpenLedger);
    let both = start_both::<Views, StandingFolds>(ledger_dir)?;
    cost.lap(Phase::FoldTail {
        lines: both.checked,
        from: both.from,
    });
    log.write(
        Level::Effect,
        Site {
            run: RunId::CITY,
            seq: both.first.last_seq().unwrap_or(Seq::FIRST),
            module: "accounting::worker",
        },
        &format!(
            "the views {}; the standing {}",
            both.first.from, both.second.from
        ),
    );
    let cut = cut(ledger_dir, &both.second);
    cost.lap(Phase::CutStanding);
    let standing = both.second.folded.settle(cut)?;
    Ok((both.first, (ledger, report, standing)))
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
