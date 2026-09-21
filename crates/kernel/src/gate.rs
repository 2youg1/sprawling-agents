// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The doors. This module is the library-wide sole producer of gate
//! refusal codes: every Deny goes through `AxError::refusal` with the
//! three mandatory parts. Boundary feedback beats opening sermons — the
//! refusal is the teaching.
//!
//! **A door answers Allow or Deny and never asks a person.** An action
//! a person would have to approve is either one the rules already
//! allow or one they already refuse, and both answers are written down
//! where a person can change them: `CONFIG.toml` for budgets and
//! trusted connectors, the building's `BUILDING.md` for write domains
//! and egress. A door that escalated would be a door whose default
//! answer gets clicked through, which is the same as no door.
//!
//! Separate functions, not one fat envelope: each door reads its own
//! inputs, the effect layer routes by the `Effect` field (5.2), and
//! kani can walk each door's space without multiplying the others'.

use crate::error::AxError;

mod discard;
mod domain;
mod egress;
mod spawn;
mod undoable;

pub use discard::discard;
pub use domain::{domain, reach};
pub use egress::{EgressAllowlist, EgressOutcome, EgressTarget, egress, egress_target};
pub use spawn::spawn;
pub use undoable::{ConnectorCall, reaches_the_undoable, undoable};

/// Deliberately exhaustive: every caller decides both ways.
#[derive(Debug)]
pub enum GateOutcome {
    Allow,
    Deny { refusal: Box<AxError> },
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
}

/// Every door, in the order this module declares them.
pub const DOORS: [DoorId; 7] = [
    DoorId::Domain,
    DoorId::Reach,
    DoorId::Egress,
    DoorId::EgressHost,
    DoorId::Discard,
    DoorId::Spawn,
    DoorId::Undoable,
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
        }
    }
}

#[cfg(feature = "conformance")]
pub mod conformance {
    //! One refusal per door, produced by the door itself.
    //!
    //! The samples call the real functions with inputs that deny, so
    //! what the matrix judges is the refusal a run would receive rather
    //! than a copy of it written for the test.

    use std::collections::BTreeMap;

    use super::{DoorId, GateOutcome};
    use crate::address::Address;
    use crate::budget::ByteLen;
    use crate::config::SandboxLimits;
    use crate::delegation::{DelegateKind, Depth};
    use crate::discard::{Discard, DiscardRequest, Restoration};
    use crate::error::AxError;
    use crate::locator::Locator;
    use crate::secret::SecretSpan;
    use crate::taint::{TaintSet, TaintSource};
    use crate::tool::{ServerLabel, ToolName};
    use crate::write_domain::WriteDomain;

    /// Whatever this door refuses, as it refuses it.
    ///
    /// # Errors
    /// Returns the door's own refusal. The `Ok` side carries the door
    /// that answered Allow when its sample was meant to deny, which is
    /// a defect in this function rather than in the door.
    pub fn deny_sample(door: DoorId) -> Result<AxError, DoorId> {
        let refused = match door {
            DoorId::Domain => domain_sample(),
            DoorId::Reach => reach_sample(),
            DoorId::Egress => egress_sample(),
            DoorId::EgressHost => egress_host_sample(),
            DoorId::Discard => discard_sample(),
            DoorId::Spawn => spawn_sample(),
            DoorId::Undoable => undoable_sample(),
        };
        refused.ok_or(door)
    }

    fn refusal_of(outcome: GateOutcome) -> Option<AxError> {
        match outcome {
            GateOutcome::Allow => None,
            GateOutcome::Deny { refusal } => Some(*refusal),
        }
    }

    fn refusal_of_egress(outcome: super::EgressOutcome) -> Option<AxError> {
        match outcome {
            super::EgressOutcome::Allow { .. } => None,
            super::EgressOutcome::Deny { refusal } => Some(*refusal),
        }
    }

    fn one_room() -> Option<WriteDomain> {
        WriteDomain::new(vec![Address::parse("b1/room").ok()?]).ok()
    }

    fn domain_sample() -> Option<AxError> {
        let elsewhere = Address::parse("b2/other.md").ok()?;
        refusal_of(super::domain(&one_room()?, &elsewhere, &TaintSet::empty()))
    }

    fn reach_sample() -> Option<AxError> {
        let elsewhere = Address::parse("b2").ok()?;
        refusal_of(super::reach(&one_room()?, &elsewhere, &TaintSet::empty()))
    }

    fn egress_sample() -> Option<AxError> {
        let span = SecretSpan {
            start: 0,
            len: 40,
            provider: Some("anthropic"),
        };
        refusal_of_egress(super::egress(
            std::slice::from_ref(&span),
            &super::EgressTarget::Public {
                host: "x.io".to_owned(),
            },
            false,
        ))
    }

    fn egress_host_sample() -> Option<AxError> {
        refusal_of_egress(super::egress_target(
            &super::EgressAllowlist::new(vec!["example.com".to_owned()]),
            &super::EgressTarget::Public {
                host: "pastebin.test".to_owned(),
            },
        ))
    }

    fn discard_sample() -> Option<AxError> {
        let unplanned = DiscardRequest::Unplanned {
            paths: vec![Address::parse("b/x.md").ok()?],
            taint: TaintSet::empty(),
            total_bytes: ByteLen::new(1),
        };
        refusal_of(super::discard(&unplanned, "delete b/x.md"))
    }

    fn spawn_sample() -> Option<AxError> {
        refusal_of(super::spawn(Depth::Delegated, &DelegateKind::Resident))
    }

    fn undoable_sample() -> Option<AxError> {
        let label = ServerLabel::parse("desk").ok()?;
        let tool = ToolName::parse("desk_desktop_act").ok()?;
        let untrusting = SandboxLimits {
            trusted: Vec::new(),
            ..SandboxLimits::default()
        };
        refusal_of(super::undoable(
            &super::ConnectorCall {
                label: &label,
                tool: &tool,
            },
            &untrusting,
            &TaintSet::empty(),
        ))
    }

    /// Which doors a taint set alone turns into a refusal.
    ///
    /// The one place an effect derived from outside content is stopped
    /// is [`super::undoable`]; the discard door stops it too, through
    /// `DiscardVerdict`. A caller that wants to know whether taint is
    /// wired at all asks here instead of grepping.
    #[must_use]
    pub fn taint_readers() -> BTreeMap<DoorId, bool> {
        let source = TaintSource::new("web:evil");
        let tainted = source.map_or_else(TaintSet::empty, TaintSet::of);
        let mut readers = BTreeMap::new();
        readers.insert(DoorId::Undoable, undoable_taint_denies(&tainted));
        readers.insert(DoorId::Discard, discard_taint_denies(&tainted));
        readers
    }

    fn undoable_taint_denies(tainted: &TaintSet) -> bool {
        let Ok(label) = ServerLabel::parse("desk") else {
            return false;
        };
        let Ok(tool) = ToolName::parse("desk_desktop_act") else {
            return false;
        };
        let trusting = SandboxLimits {
            trusted: vec![label.clone()],
            ..SandboxLimits::default()
        };
        refusal_of(super::undoable(
            &super::ConnectorCall {
                label: &label,
                tool: &tool,
            },
            &trusting,
            tainted,
        ))
        .is_some()
    }

    fn discard_taint_denies(tainted: &TaintSet) -> bool {
        let Ok(path) = Address::parse("b/x.md") else {
            return false;
        };
        let Ok(locator) = Locator::parse(&format!("file:b/x.md@{}", "ab".repeat(20))) else {
            return false;
        };
        let Ok(planned) = Discard::new(
            vec![path],
            Restoration::Tracked(locator),
            tainted.clone(),
            ByteLen::new(1),
        ) else {
            return false;
        };
        refusal_of(super::discard(
            &DiscardRequest::Planned(planned),
            "delete b/x.md",
        ))
        .is_some()
    }
}

#[cfg(all(test, feature = "conformance"))]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::{DOORS, DoorId, conformance};

    /// Every door in the roster refuses something, and says all three
    /// parts when it does. Adding a door adds a row here by adding an
    /// arm to `deny_sample`, which does not compile until it is written.
    #[test]
    fn every_door_in_the_roster_has_a_refusal_with_three_parts() {
        for door in DOORS {
            let refusal = conformance::deny_sample(door)
                .unwrap_or_else(|door| panic!("{}: the sample allowed", door.as_str()));
            let parts = refusal
                .gate()
                .unwrap_or_else(|| panic!("{}: refusal without the three parts", door.as_str()));
            assert!(!parts.rule().is_empty(), "{}: empty rule", door.as_str());
            assert!(
                !parts.violation().is_empty(),
                "{}: empty violation",
                door.as_str()
            );
            assert!(
                parts.alternative().len() > 12,
                "{}: the alternative must direct the next action",
                door.as_str()
            );
        }
    }

    /// The roster names each door once, and the names are the function
    /// names a reader greps for.
    #[test]
    fn the_roster_holds_every_door_exactly_once() {
        let mut names: Vec<&str> = DOORS.iter().map(|door| door.as_str()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "a door is listed twice");
        let mut all: Vec<DoorId> = DOORS.to_vec();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), before);
    }

    /// Taint reaches a decision rather than a sentence in a refusal:
    /// both doors that see a taint set refuse on it (C15).
    #[test]
    fn the_doors_that_see_taint_refuse_on_it() {
        for (door, denies) in conformance::taint_readers() {
            assert!(
                denies,
                "{}: taint changed no verdict, so C15 is a false branch there",
                door.as_str()
            );
        }
    }
}
