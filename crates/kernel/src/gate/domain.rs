// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::address::Address;
use crate::error::{AxCode, AxError, GateRefusal};
use crate::taint::TaintSet;
use crate::write_domain::{DomainVerdict, WriteDomain};

use super::GateOutcome;

pub fn domain(domain: &WriteDomain, target: &Address, taint: &TaintSet) -> GateOutcome {
    match domain.admits(target) {
        DomainVerdict::Within => GateOutcome::Allow,
        DomainVerdict::Outside { prefixes } => {
            let mut violation = format!("target {} is outside the write domain", target.as_str());
            if !taint.is_empty() {
                violation.push_str(&format!(
                    "; the action derives from {} external source(s)",
                    taint.len()
                ));
            }
            let alternative = if prefixes.is_empty() {
                "this actor writes nowhere; read, or hand the change to an actor with a domain"
                    .to_owned()
            } else {
                format!("write under one of: {}", prefixes.join(", "))
            };
            GateOutcome::Deny {
                refusal: Box::new(
                    AxError::refusal(
                        AxCode::OutsideWriteDomain,
                        "write file",
                        target.as_str(),
                        GateRefusal::new(
                            "writes land inside the write domain",
                            violation,
                            alternative,
                        ),
                    )
                    .with_nearby(prefixes),
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
    use super::*;
    use crate::taint::TaintSource;

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
    fn the_domain_door_refuses_with_prefixes_as_nearby() {
        let wd = WriteDomain::new(vec![Address::parse("b1/room").unwrap()]).unwrap();
        let inside = Address::parse("b1/room/notes.md").unwrap();
        assert!(matches!(
            domain(&wd, &inside, &TaintSet::empty()),
            GateOutcome::Allow
        ));
        let outside = Address::parse("b2/other.md").unwrap();
        let outcome = domain(&wd, &outside, &TaintSet::empty());
        assert_three_parts(&outcome, AxCode::OutsideWriteDomain);
        let GateOutcome::Deny { refusal } = outcome else {
            panic!("refusal shape asserted above")
        };
        assert_eq!(refusal.nearby(), ["b1/room"]);
        // Tainted violations name their provenance in the violation part.
        let tainted = TaintSet::of(TaintSource::new("web:evil").unwrap());
        let outcome = domain(&wd, &outside, &tainted);
        let GateOutcome::Deny { refusal } = outcome else {
            panic!("refusal shape asserted above")
        };
        assert!(
            refusal
                .gate()
                .unwrap()
                .violation()
                .contains("external source")
        );
    }
}
