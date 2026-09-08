// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the executor supplies at a phase change, and what a phase
//! change answers back.

use kernel::EventRef;

/// Boundary snapshot, supplied by the executor at every phase change.
/// `Cancel` ends the turn at the boundary; `Steer` records and advances
/// (the executor folds the text into its `Window`).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interrupt {
    None,
    Cancel,
    Steer { source: String, text: String },
}

/// A phase change either advances or ends the turn at the boundary.
/// Deliberately exhaustive: a new outcome must force every executor to
/// decide, not fall through a catch-all (14.3's non_exhaustive rule
/// covers wire enums, not verdicts).
#[derive(Debug)]
pub enum PhaseOutcome<Next> {
    Advanced(Next),
    Cancelled(TurnCancelled),
}

/// The turn ended at a boundary: `cancel_received` is on the ledger and
/// its ref is the last entry. Freezing (handoff + run_frozen) is the run
/// loop's move, not the turn's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnCancelled {
    pub(super) refs: Vec<EventRef>,
}

impl TurnCancelled {
    pub fn refs(&self) -> &[EventRef] {
        &self.refs
    }
}
