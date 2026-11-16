// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::AxError;

mod attach;
mod command;
mod discard;
mod domain;
mod egress;
mod spawn;
mod undoable;

pub use attach::attach;
pub use command::command;
pub use discard::discard;
pub use domain::{domain, reach, replacing};
pub use egress::{
    EgressAllowlist, EgressOutcome, EgressTarget, egress, egress_target, host_of, target_of,
};
pub use spawn::spawn;
pub use undoable::{ConnectorCall, reaches_the_undoable, undoable};

#[derive(Debug)]
pub enum GateOutcome {
    Allow,
    Deny {
        refusal: Box<AxError>,
    },

    Ask {
        question: Box<AxError>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DoorId {

    Domain,

    Reach,

    Egress,

    EgressHost,

    Discard,

    Spawn,

    Undoable,

    Attach,

    Command,

    Replacing,
}

pub const DOORS: [DoorId; 10] = [
    DoorId::Domain,
    DoorId::Reach,
    DoorId::Egress,
    DoorId::EgressHost,
    DoorId::Discard,
    DoorId::Spawn,
    DoorId::Undoable,
    DoorId::Attach,
    DoorId::Command,
    DoorId::Replacing,
];

impl DoorId {

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
            DoorId::Command => "command",
            DoorId::Replacing => "replacing",
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
