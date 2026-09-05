// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::address::Address;
use crate::approval::ApprovalClass;
use crate::delegation::{DelegateKind, DelegationVerdict, Depth, admit as admit_delegation};
use crate::discard::{DenyReason, DiscardRequest, DiscardVerdict, EscalateReason, decide};
use crate::error::{AxCode, AxError, GateRefusal};
use crate::locator::Locator;
use crate::registry::Registry;
use crate::taint::TaintSet;

use super::item;
use super::{GateContext, GateOutcome};

pub fn discard(
    req: &DiscardRequest,
    registry: &Registry,
    ctx: &GateContext,
    action_desc: &str,
    artifact: &Locator,
) -> GateOutcome {
    match decide(req, registry) {
        DiscardVerdict::Allow => GateOutcome::Allow,
        DiscardVerdict::Escalate { reason } => {
            let (detail, tainted) = match reason {
                EscalateReason::Tainted => ("tainted", true),
                EscalateReason::FilesOverMax => ("files_over_max", false),
                EscalateReason::BytesOverMax => ("bytes_over_max", false),
                EscalateReason::RegistryAsset => ("registry_asset", false),
            };
            GateOutcome::Escalate {
                item: item(
                    ctx,
                    ApprovalClass::DiscardEscalate,
                    detail.to_owned(),
                    action_desc.to_owned(),
                    artifact.clone(),
                    tainted,
                ),
            }
        }
        DiscardVerdict::Deny { reason } => {
            let DenyReason::NoRestoration = reason;
            GateOutcome::Deny {
                refusal: Box::new(AxError::refusal(
                    AxCode::DiscardIrreversible,
                    "discard files",
                    action_desc,
                    GateRefusal::new(
                        "every discard carries a resolvable restoration (C14)",
                        "this request names no restoration plan",
                        "split the batch under the thresholds, or inter the originals \
                         in CAS (Interred) and retry with that locator",
                    ),
                )),
            }
        }
    }
}

/// The Delegation door: a second agent starts because a person allowed
/// it, not because a prompt asked politely.
///
/// Always escalates. Whether this person has already said yes is the
/// caller's own record of what was granted - the same shape the write
/// door uses - and keeping the two apart is what lets one answer cover a
/// cluster instead of one call.
///
/// The cluster detail is the *asking* address rather than the room being
/// opened: the person is being asked whether this resident may hand work
/// to anybody, which is the question, and asking again per room would
/// train them to click through it.
pub fn delegation(
    ctx: &GateContext,
    asking: &Address,
    room: &Address,
    artifact: &Locator,
    taint: &TaintSet,
) -> GateOutcome {
    GateOutcome::Escalate {
        item: item(
            ctx,
            ApprovalClass::Delegation,
            asking.as_str().to_owned(),
            format!(
                "{} wants to hand work to another agent, in {}",
                asking.as_str(),
                room.as_str()
            ),
            artifact.clone(),
            !taint.is_empty(),
        ),
    }
}

/// The Governance door: what governs a scope changes because a person
/// changed it.
///
/// Always escalates, for the same reason `delegation` does, and the
/// description carries a bounded excerpt of the proposal - a person
/// asked to allow a rewrite of the rules and shown only the word
/// "rewrite" is being asked to guess.
pub fn govern(
    ctx: &GateContext,
    asking: &Address,
    scope: &Address,
    proposal: &str,
    artifact: &Locator,
    taint: &TaintSet,
) -> GateOutcome {
    let excerpt: String = proposal.chars().take(PROPOSAL_EXCERPT).collect();
    let ellipsis = if proposal.chars().count() > PROPOSAL_EXCERPT {
        " ..."
    } else {
        ""
    };
    GateOutcome::Escalate {
        item: item(
            ctx,
            ApprovalClass::Governance,
            asking.as_str().to_owned(),
            format!(
                "{} wants to rewrite the rules of {}:
{excerpt}{ellipsis}",
                asking.as_str(),
                scope.as_str()
            ),
            artifact.clone(),
            !taint.is_empty(),
        ),
    }
}

/// How much of a proposed governance document a person is shown in the
/// approvals list. Long enough to read the declarations that matter,
/// short enough that the list stays a list; the whole text is in the
/// `tool_called` line either way.
const PROPOSAL_EXCERPT: usize = 600;

/// The spawn admission: delegates do not delegate (10.1). The refusal
/// teaches the alternative instead of hiding the tool.
pub fn spawn(parent: Depth, kind: &DelegateKind) -> GateOutcome {
    match admit_delegation(parent, kind) {
        DelegationVerdict::Allow => GateOutcome::Allow,
        DelegationVerdict::Deny => GateOutcome::Deny {
            refusal: Box::new(AxError::refusal(
                AxCode::DelegationDepth,
                "spawn delegate",
                format!("{kind:?}"),
                GateRefusal::new(
                    "delegates do not delegate: one level deep",
                    "a delegated position requested a spawn",
                    "return this subtask to the resident who spawned you; \
                     that resident can delegate it",
                ),
            )),
        },
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
mod tests {
    use super::super::{GateContext, GateOutcome};
    use super::*;
    use crate::approval::ApprovalId;
    use crate::event::TimeMs;
    use crate::locator::Locator;

    fn ctx() -> GateContext {
        GateContext {
            actor: "worker@sim.1".into(),
            now: TimeMs::new(42),
            item_id: ApprovalId::new("item-7").unwrap(),
        }
    }

    fn artifact() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "aa".repeat(32))).unwrap()
    }

    fn assert_three_parts(outcome: &GateOutcome, code: AxCode) {
        match outcome {
            GateOutcome::Deny { refusal } => {
                assert_eq!(refusal.code(), &code);
                let gate = refusal.gate().expect("gate refusals carry three parts");
                assert!(!gate.rule().is_empty());
                assert!(!gate.violation().is_empty());
                assert!(!gate.alternative().is_empty());
            }
            _ => panic!("expected a refusal"),
        }
    }

    #[test]
    fn the_discard_door_maps_verdicts_and_teaches_the_alternative() {
        use crate::budget::ByteLen;
        let registry = Registry::new();
        let unplanned = DiscardRequest::Unplanned {
            paths: vec![Address::parse("b/x.md").unwrap()],
            taint: TaintSet::empty(),
            total_bytes: ByteLen::new(10),
        };
        let outcome = discard(&unplanned, &registry, &ctx(), "delete b/x.md", &artifact());
        assert_three_parts(&outcome, AxCode::DiscardIrreversible);
        let GateOutcome::Deny { refusal } = outcome else {
            panic!("refusal shape asserted above")
        };
        assert!(refusal.gate().unwrap().alternative().contains("Interred"));
    }

    #[test]
    fn the_spawn_door_teaches_the_way_back_up() {
        assert!(matches!(
            spawn(Depth::Root, &DelegateKind::Ephemeral),
            GateOutcome::Allow
        ));
        let outcome = spawn(Depth::Delegated, &DelegateKind::Ephemeral);
        assert_three_parts(&outcome, AxCode::DelegationDepth);
    }
}
