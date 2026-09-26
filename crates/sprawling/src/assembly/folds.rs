// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Everything a worker inherits from a history it did not write.

use std::path::Path;

use kernel::AxError;

// The governance fold lives in `views`, where the reading side keeps
// one too. Named here so the worker that judges from it reads under
// the same name a page is answered under.
pub(super) use crate::views::Governance;
use crate::views::known_records;

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
    /// A line is parsed once, by the per-line check, and shown to each fold. Verification
    /// stays where it was: a history that does not verify is not one any
    /// of these three views may be built from.
    ///
    /// # Errors
    /// Propagates chain verification and whatever a fold says about a
    /// payload it cannot read.
    pub(crate) fn fold(ledger_dir: &Path) -> Result<Standing, AxError> {
        let mut book = gateway::EndpointBook::new();
        let mut governance = Governance::empty();
        let mut collaboration = CollaborationFold::default();
        let mut entrance = Entrance::default();
        let mut expiries = Expiries::default();
        let mut origins = SessionOrigins::default();
        if ledger_dir.exists() {
            let verified = runtime::replay::verify_ledger_dir(ledger_dir)?;
            for record in known_records(&verified) {
                book.apply(record)?;
                governance.absorb(record.kind(), record.run(), record.addr(), record.data())?;
                collaboration.absorb(record)?;
                entrance.absorb(record.data());
                expiries.absorb(record.kind(), record.data())?;
                origins.absorb(record.kind(), record.addr(), record.data())?;
            }
        }
        Ok(Standing {
            book,
            governance,
            collaboration: collaboration.settle()?,
            entrance,
            expiries,
            origins,
        })
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
