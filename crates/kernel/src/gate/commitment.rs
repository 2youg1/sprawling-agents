// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::approval::ApprovalClass;
use crate::error::{AxCode, AxError};
use crate::locator::Locator;
use crate::taint::TaintSet;

use super::item;
use super::{GateContext, GateOutcome};

pub enum CommitmentDecision {
    Approved,
    Denied,
}

/// The Commitment door: outward promises pass a human, always (9.1's
/// first must-pass class; no Policy variant can waive it). No ruling
/// pre-blocks; a denial is final for this action.
pub fn commitment(
    decision: Option<&CommitmentDecision>,
    taint: &TaintSet,
    ctx: &GateContext,
    action_desc: &str,
    artifact: &Locator,
) -> GateOutcome {
    match decision {
        None => GateOutcome::Escalate {
            item: item(
                ctx,
                ApprovalClass::Commitment,
                action_desc.to_owned(),
                action_desc.to_owned(),
                artifact.clone(),
                !taint.is_empty(),
            ),
        },
        Some(CommitmentDecision::Approved) => GateOutcome::Allow,
        Some(CommitmentDecision::Denied) => GateOutcome::Deny {
            refusal: Box::new(
                AxError::failure(AxCode::ApprovalDenied, "commit outward", action_desc)
                    .with_recovery(
                        "the human refused this commitment; change the plan or ask \
                         with a different, clearly scoped item",
                    ),
            ),
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

    #[test]
    fn the_commitment_door_pre_blocks_and_respects_denial() {
        let outcome = commitment(
            None,
            &TaintSet::empty(),
            &ctx(),
            "send release mail",
            &artifact(),
        );
        match outcome {
            GateOutcome::Escalate { item } => {
                assert_eq!(item.cluster_key.class, ApprovalClass::Commitment);
            }
            _ => panic!("no ruling pre-blocks"),
        }
        assert!(matches!(
            commitment(
                Some(&CommitmentDecision::Approved),
                &TaintSet::empty(),
                &ctx(),
                "send release mail",
                &artifact()
            ),
            GateOutcome::Allow
        ));
        match commitment(
            Some(&CommitmentDecision::Denied),
            &TaintSet::empty(),
            &ctx(),
            "send release mail",
            &artifact(),
        ) {
            GateOutcome::Deny { refusal } => {
                assert_eq!(refusal.code(), &AxCode::ApprovalDenied);
                assert!(refusal.gate().is_none(), "not a gate-carrier code");
            }
            _ => panic!("denied is final"),
        }
    }
}
