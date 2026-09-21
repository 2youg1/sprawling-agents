// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The doors. This module is the library-wide sole producer of gate
//! refusal codes: every Deny goes through `AxError::refusal` with the
//! three mandatory parts. Boundary feedback beats opening sermons — the
//! refusal is the teaching.
//!
//! **A door answers Allow or Deny, and never asks a person** — with one
//! named exception. An action a person would have to approve is either
//! one the rules already allow or one they already refuse, and both
//! answers are written down where a person can change them: `CONFIG.toml`
//! for budgets and trusted connectors, the building's `RULES.toml` for
//! write domains and egress. A door that escalated would be a door whose
//! default answer gets clicked through, which is the same as no door.
//!
//! [`attach`] is the exception, and it is the only one: what it would
//! grant is the reading right over every login a person's own browser
//! holds, so no rule of the city's can answer it. The person's action is
//! the answer, and the recovery sentence is the question. Whether a door
//! asks is a property of the door rather than a switch: the roster test
//! asserts that exactly one door returns [`GateOutcome::Ask`].
//!
//! Separate functions, not one fat envelope: each door reads its own
//! inputs, the effect layer routes by the `Effect` field (5.2), and
//! kani can walk each door's space without multiplying the others'.

use crate::error::AxError;

mod attach;
mod discard;
mod domain;
mod egress;
mod spawn;
mod undoable;

pub use attach::attach;
pub use discard::discard;
pub use domain::{domain, reach};
pub use egress::{
    EgressAllowlist, EgressOutcome, EgressTarget, egress, egress_target, host_of, target_of,
};
pub use spawn::spawn;
pub use undoable::{ConnectorCall, reaches_the_undoable, undoable};

/// Deliberately exhaustive, and the third arm is deliberately rare.
///
/// Every caller decides all three ways, so a door that starts asking a
/// person is a compile error at every caller rather than a behaviour
/// that changes underneath them. Exactly one door answers `Ask`
/// ([`attach`]); the refusal matrix asserts that count.
#[derive(Debug)]
pub enum GateOutcome {
    Allow,
    Deny {
        refusal: Box<AxError>,
    },
    /// The door cannot answer, and the person can. The error carries
    /// the question: `E_APPROVAL_PENDING`, the three gate parts, and a
    /// recovery sentence naming the action only the person can take.
    Ask {
        question: Box<AxError>,
    },
}

/// The roster of doors, as data.
///
/// The refusal conformance matrix walks [`DOORS`] rather than naming
/// doors one by one, so a door added to this enum is a door the matrix
/// judges: [`conformance::deny_sample`] matches exhaustively, and a new
/// arm without a sample does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DoorId {
    /// [`domain`] — may this file be written.
    Domain,
    /// [`reach`] — may this area be worked in at all.
    Reach,
    /// [`egress`] — may these bytes leave.
    Egress,
    /// [`egress_target`] — may this host be reached.
    EgressHost,
    /// [`discard`] — may these files go.
    Discard,
    /// [`spawn`] — may this position hand work down.
    Spawn,
    /// [`undoable`] — may this connector reach what nothing here can
    /// take back.
    Undoable,
    /// [`attach`] — may a run drive the browser a person is already
    /// using, with that person's logins.
    Attach,
}

/// Every door, in the order this module declares them.
pub const DOORS: [DoorId; 8] = [
    DoorId::Domain,
    DoorId::Reach,
    DoorId::Egress,
    DoorId::EgressHost,
    DoorId::Discard,
    DoorId::Spawn,
    DoorId::Undoable,
    DoorId::Attach,
];

impl DoorId {
    /// The door's name in a refusal report, which is the name of the
    /// function that produced it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DoorId::Domain => "domain",
            DoorId::Reach => "reach",
            DoorId::Egress => "egress",
            DoorId::EgressHost => "egress_target",
            DoorId::Discard => "discard",
            DoorId::Spawn => "spawn",
            DoorId::Undoable => "undoable",
            DoorId::Attach => "attach",
        }
    }
}

#[cfg(feature = "conformance")]
pub mod conformance;

#[cfg(all(test, feature = "conformance"))]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests;
