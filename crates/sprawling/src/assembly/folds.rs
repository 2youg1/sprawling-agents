// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a worker inherits from a history it did not write.

use std::path::Path;

use kernel::AxError;
use kernel::EventRecord;
use runtime::replay::{VerifiedLedger, VerifiedLine};

// The governance fold lives in `views`, where the reading side keeps
// one too. Named here so the worker that judges from it reads under
// the same name a page is answered under.
pub(super) use crate::views::Governance;
use crate::views::Views;

use super::{Entrance, Expiries};

mod collaboration;
mod session;
mod snapshot_start;
mod standing_start;
mod views_start;

pub(super) use collaboration::{Collaboration, INBOX_CAPACITY, new_inbox};
pub(super) use session::SessionOrigins;
use snapshot_start::SnapshotFold;
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
    /// When each subscription credential stops working. On the same
    /// pass for the same reason, and folded at all because a worker
    /// that read it only from its own process renewed nothing after a
    /// restart and met each expiry as a 401 mid-run.
    pub(super) expiries: Expiries,
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
    /// snapshot cut at the last line folded (sprawling-SPEC 8-92).
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
        let started = snapshot_start::start_audited::<StandingFolds>(ledger_dir)?;
        let cut = snapshot_start::cut(ledger_dir, &started);
        started.folded.settle(cut)
    }
}

/// The views of the ledger on disk, from its snapshot when one fits
/// (sprawling-SPEC 8-91). A reader that serves nothing: it cuts no
/// snapshot, so a one-shot query writes nothing to disk.
///
/// The whole chain is audited first, because no background audit runs
/// beside a one-shot read and the snapshot's fit checks only the line at
/// its seq: without the audit a line edited before that seq would be
/// answered from.
///
/// # Errors
/// The audit's reason when the chain is broken or cannot be read, and
/// the verification failures of the lines it folds; a city whose history
/// does not verify is not one whose views should be served.
pub(crate) fn rebuild_views(ledger_dir: &Path) -> Result<Views, AxError> {
    snapshot_start::start_audited::<Views>(ledger_dir).map(|started| started.folded)
}

/// The records the per-line check already parsed, in ledger order.
///
/// A line the check let through as ignorable carries a kind this build
/// has no record for, so no fold is shown it; parsing the raw bytes a
/// second time would refuse exactly that line and turn a history that
/// verifies into a city that cannot start.
fn known_records(verified: &VerifiedLedger) -> impl Iterator<Item = &EventRecord> {
    verified.lines().iter().filter_map(|line| match line {
        VerifiedLine::Known { record, .. } => Some(record),
        VerifiedLine::IgnoredUnknown { .. } => None,
    })
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
