// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One refusal per door, produced by the door itself.
//! The part `crates/kernel/spec/Gate.lean` specifies this module with the
//! rest of `kernel::gate`.
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
use crate::locator::Locator;
use crate::secret::SecretSpan;
use crate::taint::{TaintSet, TaintSource};
use crate::tool::{ServerLabel, ToolName};
use crate::write_domain::{WriteDomain, WriteLimit};

/// Whatever this door answers when its sample calls it.
///
/// The samples call the real functions with inputs that deny (or,
/// for the one asking door, inputs the person must answer), so what
/// the matrix judges is the answer a run would receive rather than a
/// copy of it written for the test.
///
/// # Errors
/// Returns the door when it answered Allow, which is a defect in
/// this function rather than in the door: every sample is built to
/// provoke something other than Allow.
pub fn sample(door: DoorId) -> Result<GateOutcome, DoorId> {
    let answered = match door {
        DoorId::Domain => domain_sample(),
        DoorId::Reach => reach_sample(),
        DoorId::Egress => egress_sample(),
        DoorId::EgressHost => egress_host_sample(),
        DoorId::Discard => discard_sample(),
        DoorId::Spawn => spawn_sample(),
        DoorId::Undoable => undoable_sample(),
        DoorId::Attach => attach_sample(),
        DoorId::Command => command_sample(),
        DoorId::Replacing => replacing_sample(),
    };
    answered.ok_or(door)
}

fn outcome_of(outcome: GateOutcome) -> Option<GateOutcome> {
    match outcome {
        GateOutcome::Allow => None,
        answered @ (GateOutcome::Deny { .. } | GateOutcome::Ask { .. }) => Some(answered),
    }
}

fn refusal_of_egress(outcome: super::EgressOutcome) -> Option<GateOutcome> {
    match outcome {
        super::EgressOutcome::Allow { .. } => None,
        super::EgressOutcome::Deny { refusal } => Some(GateOutcome::Deny { refusal }),
    }
}

fn one_room() -> Option<WriteDomain> {
    WriteDomain::new(vec![Address::parse("b1/room").ok()?]).ok()
}

fn domain_sample() -> Option<GateOutcome> {
    let elsewhere = Address::parse("b2/other.md").ok()?;
    outcome_of(super::domain(&one_room()?, &elsewhere, &TaintSet::empty()))
}

fn replacing_sample() -> Option<GateOutcome> {
    let existing = Address::parse("b1/room/notes.md").ok()?;
    outcome_of(super::replacing(WriteLimit::Create, &existing))
}

fn reach_sample() -> Option<GateOutcome> {
    let elsewhere = Address::parse("b2").ok()?;
    outcome_of(super::reach(&one_room()?, &elsewhere, &TaintSet::empty()))
}

fn egress_sample() -> Option<GateOutcome> {
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

fn egress_host_sample() -> Option<GateOutcome> {
    refusal_of_egress(super::egress_target(
        &super::EgressAllowlist::new(vec!["example.com".to_owned()]),
        &super::EgressTarget::Public {
            host: "pastebin.test".to_owned(),
        },
    ))
}

fn command_sample() -> Option<GateOutcome> {
    let tainted = TaintSet::of(TaintSource::new("web:evil")?);
    outcome_of(super::command(&tainted))
}

fn discard_sample() -> Option<GateOutcome> {
    let unplanned = DiscardRequest::Unplanned {
        paths: vec![Address::parse("b/x.md").ok()?],
        taint: TaintSet::empty(),
        total_bytes: ByteLen::new(1),
    };
    outcome_of(super::discard(&unplanned, "delete b/x.md"))
}

fn spawn_sample() -> Option<GateOutcome> {
    outcome_of(super::spawn(Depth::Delegated, &DelegateKind::Resident))
}

fn undoable_sample() -> Option<GateOutcome> {
    let label = ServerLabel::parse("desk").ok()?;
    let tool = ToolName::parse("desk_desktop_act").ok()?;
    let untrusting = SandboxLimits {
        trusted: Vec::new(),
        ..SandboxLimits::default()
    };
    outcome_of(super::undoable(
        &super::ConnectorCall {
            label: &label,
            tool: &tool,
        },
        &untrusting,
        &TaintSet::empty(),
    ))
}

/// The one door that asks: a building that enabled the tool and
/// declared no address is the state the person's next action
/// resolves.
fn attach_sample() -> Option<GateOutcome> {
    outcome_of(super::attach(None))
}

/// Which doors a taint set alone turns into a refusal: for each door
/// that reads taint, whether the same input is refused with a taint set
/// and not refused without one.
///
/// The one place an effect derived from outside content is stopped
/// is [`super::undoable`]; the discard door stops it too, through
/// `DiscardVerdict`. A caller that wants to know whether taint is
/// wired at all asks here instead of grepping. Both halves are asked,
/// because a door that refused every input would refuse a tainted one
/// too and still read nothing.
#[must_use]
pub fn taint_readers() -> BTreeMap<DoorId, bool> {
    let source = TaintSource::new("web:evil");
    let tainted = source.map_or_else(TaintSet::empty, TaintSet::of);
    let clean = TaintSet::empty();
    let command_denies =
        |taint: &TaintSet| matches!(super::command(taint), GateOutcome::Deny { .. });
    let mut readers = BTreeMap::new();
    readers.insert(
        DoorId::Undoable,
        undoable_taint_denies(&tainted) && !undoable_taint_denies(&clean),
    );
    readers.insert(
        DoorId::Discard,
        discard_taint_denies(&tainted) && !discard_taint_denies(&clean),
    );
    readers.insert(
        DoorId::Command,
        command_denies(&tainted) && !command_denies(&clean),
    );
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
    matches!(
        super::undoable(
            &super::ConnectorCall {
                label: &label,
                tool: &tool,
            },
            &trusting,
            tainted,
        ),
        GateOutcome::Deny { .. }
    )
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
    matches!(
        super::discard(&DiscardRequest::Planned(planned), "delete b/x.md"),
        GateOutcome::Deny { .. }
    )
}
