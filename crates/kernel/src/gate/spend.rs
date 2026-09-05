// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::approval::ApprovalClass;
use crate::budget::{BudgetLadder, BudgetLayer, BudgetUse};
use crate::locator::Locator;
use crate::taint::TaintSet;

use super::item;
use super::{GateContext, GateOutcome};

pub fn spend(
    ladder: &BudgetLadder,
    cost: &BudgetUse,
    taint: &TaintSet,
    ctx: &GateContext,
    artifact: &Locator,
) -> GateOutcome {
    match crate::budget::admit_spend(ladder, cost) {
        crate::budget::SpendVerdict::Admit => GateOutcome::Allow,
        crate::budget::SpendVerdict::Exhausted { layer } => {
            let layer_name = match layer {
                BudgetLayer::City => "city",
                BudgetLayer::Building => "building",
                BudgetLayer::Run => "run",
            };
            GateOutcome::Escalate {
                item: item(
                    ctx,
                    ApprovalClass::BudgetLimit,
                    layer_name.to_owned(),
                    format!("continue past the {layer_name} budget"),
                    artifact.clone(),
                    !taint.is_empty(),
                ),
            }
        }
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
    use crate::approval::ApprovalSource;
    use crate::budget::{BudgetCap, BudgetLevel, Tokens, UsdMicros};
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
    fn the_spend_door_escalates_exhaustion_as_a_budget_item() {
        let level = |cap: u64, used: u64| BudgetLevel {
            cap: BudgetCap {
                usd: UsdMicros::new(cap),
                tokens: Tokens::new(1_000_000),
            },
            used: BudgetUse {
                usd: UsdMicros::new(used),
                tokens: Tokens::new(0),
            },
        };
        let ladder = BudgetLadder {
            city: level(1000, 0),
            building: level(1000, 0),
            run: level(10, 10),
        };
        let cost = BudgetUse {
            usd: UsdMicros::new(1),
            tokens: Tokens::new(1),
        };
        match spend(&ladder, &cost, &TaintSet::empty(), &ctx(), &artifact()) {
            GateOutcome::Escalate { item } => {
                assert_eq!(item.cluster_key.class, ApprovalClass::BudgetLimit);
                assert_eq!(item.cluster_key.detail, "run");
                assert_eq!(item.source, ApprovalSource::Gate);
                assert!(!item.tainted);
            }
            _ => panic!("exhaustion escalates"),
        }
        let roomy = BudgetLadder {
            city: level(1000, 0),
            building: level(1000, 0),
            run: level(1000, 0),
        };
        assert!(matches!(
            spend(&roomy, &cost, &TaintSet::empty(), &ctx(), &artifact()),
            GateOutcome::Allow
        ));
    }
}
