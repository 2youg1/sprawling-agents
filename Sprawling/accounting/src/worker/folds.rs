// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{AxError, RunId, Seq};
use runtime::diagnostics::{Diagnostics, Level, Site};
use storage::{JsonlLedger, OpenReport};

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

pub struct Standing {
    pub book: gateway::EndpointBook,
    pub(super) governance: Governance,
    pub(super) collaboration: Collaboration,

    pub(super) entrance: Entrance,

    pub(super) origins: SessionOrigins,

    pub(super) cut: Result<(), AxError>,
}

impl Standing {

    pub fn fold(ledger_dir: &Path) -> Result<Standing, AxError> {
        if !ledger_dir.exists() {
            return StandingFolds::empty(ledger_dir).settle(Ok(()));
        }
        let started = start_audited::<StandingFolds>(ledger_dir)?.started;
        let cut = cut(ledger_dir, &started);
        started.folded.settle(cut)
    }
}

pub(crate) type Held = (JsonlLedger, OpenReport, Standing);

pub(crate) fn fold_city(
    ledger_dir: &Path,
    now: kernel::TimeMs,
    cost: &mut OpeningCost,
    log: &mut Diagnostics,
) -> Result<(Started<Views>, Held), AxError> {

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
