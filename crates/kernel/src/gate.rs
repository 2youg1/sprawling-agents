// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The five doors and idempotent dedup. This module is
//! the library-wide sole producer of gate refusal codes: every Deny goes
//! through `AxError::refusal` with the three mandatory parts, and every
//! Escalate mints the ApprovalItem that pre-blocks the run. Boundary
//! feedback beats opening sermons — the refusal is the teaching.
//!
//! Five separate functions, not one fat envelope: each door reads its own
//! inputs, the effect layer routes by the `Effect` field (5.2), and kani
//! can walk each door's space without multiplying the others'.

use crate::approval::{ApprovalId, ApprovalItem};
use crate::error::AxError;
use crate::event::TimeMs;

mod commitment;
mod dedup;
mod domain;
mod egress;
mod govern;
mod item;

pub use commitment::{CommitmentDecision, commitment};
pub use dedup::{DedupVerdict, dedup};
pub use domain::domain;
pub use egress::{EgressAllowlist, EgressOutcome, EgressTarget, egress, egress_target};
pub use govern::{delegation, discard, govern, spawn};
pub(crate) use item::item;

/// What an Escalate needs to mint its item; all injected — the gate
/// samples nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateContext {
    pub actor: String,
    pub now: TimeMs,
    pub item_id: ApprovalId,
}

/// Deliberately exhaustive: every caller decides all three ways.
#[derive(Debug)]
pub enum GateOutcome {
    Allow,
    Escalate { item: ApprovalItem },
    Deny { refusal: Box<AxError> },
}
