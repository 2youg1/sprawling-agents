// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

use super::*;
use kernel::Range;

const SPEC_DRAFT: &str =
    "The city keeps one ledger.\nEvery effect\n   is recorded first.\nNothing else.\n";
const PLAN_DRAFT: &str = "Residents share one inbox.\nA duplicate signal is dropped.\n";
const EARLIER_PLAN: &str = "Residents share one inbox.\nA duplicate signal is kept.\n";

fn cite(draft: &str, quote: &str, range: Range) -> Citation {
    Citation::new(
        quote.to_owned(),
        Locator::Cas {
            hash: B3Hash::digest(draft.as_bytes()),
            range: Some(range),
        },
    )
}

fn lines(from: u64, to: u64) -> Range {
    Range::lines(from, to).unwrap()
}

fn claim(citation: Citation, plant: Plant) -> Claim {
    Claim { citation, plant }
}

/// The red team's script: two drafts, four faithful conclusions and one
/// of each planted defect.
fn script() -> Vec<Case> {
    vec![
        Case {
            draft: SPEC_DRAFT.to_owned(),
            claims: vec![
                claim(
                    cite(SPEC_DRAFT, "The city keeps one ledger.", lines(1, 1)),
                    Plant::Faithful,
                ),
                claim(
                    cite(SPEC_DRAFT, "Every effect is recorded first.", lines(2, 3)),
                    Plant::Faithful,
                ),
                claim(
                    cite(SPEC_DRAFT, "Every effect is recorded later.", lines(2, 3)),
                    Plant::Misquote,
                ),
                claim(
                    cite(SPEC_DRAFT, "Nothing else.", lines(4, 9)),
                    Plant::PastEnd,
                ),
            ],
        },
        Case {
            draft: PLAN_DRAFT.to_owned(),
            claims: vec![
                claim(
                    cite(PLAN_DRAFT, "Residents share one inbox.", lines(1, 1)),
                    Plant::Faithful,
                ),
                claim(
                    cite(PLAN_DRAFT, "A duplicate signal is dropped.", lines(2, 2)),
                    Plant::Faithful,
                ),
                claim(
                    cite(EARLIER_PLAN, "Residents share one inbox.", lines(1, 1)),
                    Plant::OtherVersion,
                ),
            ],
        },
    ]
}

#[test]
fn the_verified_arm_keeps_only_faithful_conclusions() {
    let comparison = compare(&script());
    assert_eq!(
        comparison,
        Comparison {
            unverified: Tally {
                kept_faithful: 4,
                kept_planted: 3,
                dropped_faithful: 0,
                dropped_planted: 0
            },
            verified: Tally {
                kept_faithful: 4,
                kept_planted: 0,
                dropped_faithful: 0,
                dropped_planted: 3
            },
        }
    );
    assert_eq!(comparison.verified.precision_per_mille(), Some(1000));
    assert_eq!(comparison.unverified.precision_per_mille(), Some(571));
}

#[test]
fn every_planted_defect_is_dropped_by_its_own_reading() {
    for plant in [Plant::Misquote, Plant::OtherVersion, Plant::PastEnd] {
        let alone: Vec<Case> = script()
            .into_iter()
            .map(|case| Case {
                claims: case
                    .claims
                    .into_iter()
                    .filter(|claim| claim.plant == plant)
                    .collect(),
                ..case
            })
            .collect();
        let verified = compare(&alone).verified;
        assert_eq!(
            verified,
            Tally {
                dropped_planted: 1,
                ..Tally::default()
            },
            "{plant:?} was kept by the verified arm"
        );
        assert_eq!(verified.precision_per_mille(), None);
    }
}
