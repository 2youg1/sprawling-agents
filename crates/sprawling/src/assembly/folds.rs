// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a worker inherits from a history it did not write.

use std::path::Path;

use kernel::AxError;
use kernel::EventRecord;
use memory::{JsonlLedger, OpenReport};
use runtime::replay::fold_ledger_dir;

// The governance fold lives in `views`, where the reading side keeps
// one too. Named here so the worker that judges from it reads under
// the same name a page is answered under.
pub(super) use crate::views::Governance;
use crate::views::Views;

use super::{Entrance, Expiries};

mod collaboration;
mod session;

use collaboration::CollaborationFold;
pub(super) use collaboration::{Collaboration, INBOX_CAPACITY, new_inbox};
pub(super) use session::SessionOrigins;

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
}

impl Standing {
    /// One verified pass, three folds.
    ///
    /// Until this existed the three were three functions, and opening a
    /// worker read, parsed and chain-verified the same bytes three times
    /// over to answer three questions about them. The answers never
    /// disagreed, which `what_a_worker_holds_is_what_a_restart_rebuilds`
    /// is what now holds, so the two extra passes bought nothing but the
    /// time and the memory of reading a whole history twice more.
    ///
    /// A line is parsed once, by the per-line check, and shown to each fold
    /// before the next line is read. Verification
    /// stays where it was: a history that does not verify is not one any
    /// of these three views may be built from.
    ///
    /// # Errors
    /// Propagates chain verification and whatever a fold says about a
    /// payload it cannot read.
    pub(crate) fn fold(ledger_dir: &Path) -> Result<Standing, AxError> {
        let mut standing = StandingFold::new();
        if ledger_dir.exists() {
            fold_ledger_dir(ledger_dir, |record| standing.absorb(record))?;
        }
        standing.settle()
    }
}

/// The folds a `Standing` is made of, part way through a history.
struct StandingFold {
    book: gateway::EndpointBook,
    governance: Governance,
    collaboration: CollaborationFold,
    entrance: Entrance,
    expiries: Expiries,
    origins: SessionOrigins,
}

impl StandingFold {
    fn new() -> StandingFold {
        StandingFold {
            book: gateway::EndpointBook::new(),
            governance: Governance::empty(),
            collaboration: CollaborationFold::default(),
            entrance: Entrance::default(),
            expiries: Expiries::default(),
            origins: SessionOrigins::default(),
        }
    }

    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.book.apply(record)?;
        self.governance
            .absorb(record.kind(), record.run(), record.addr(), record.data())?;
        self.collaboration.absorb(record)?;
        self.entrance.absorb(record.data());
        self.expiries.absorb(record.kind(), record.data())?;
        self.origins
            .absorb(record.kind(), record.run(), record.addr(), record.data())
    }

    fn settle(self) -> Result<Standing, AxError> {
        Ok(Standing {
            book: self.book,
            governance: self.governance,
            collaboration: self.collaboration.settle()?,
            entrance: self.entrance,
            expiries: self.expiries,
            origins: self.origins,
        })
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
/// # Errors
/// Propagates opening the ledger, chain verification, and whatever a fold
/// says about a payload it cannot read.
pub(crate) fn fold_city(
    ledger_dir: &Path,
) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError> {
    let (ledger, report) =
        JsonlLedger::open(ledger_dir, accounting::Clock::now(&super::SystemClock)?)
            .map_err(memory::MemoryError::into_ax)?;
    let mut views = Views::over(ledger_dir);
    let mut standing = StandingFold::new();
    let index = fold_ledger_dir(ledger_dir, |record| {
        views.apply(record)?;
        standing.absorb(record)
    })?;
    views.hold_index(index, ledger_dir)?;
    Ok((views, (ledger, report, standing.settle()?)))
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
