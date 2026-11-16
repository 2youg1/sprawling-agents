// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::request::{DenyReason, DiscardRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardVerdict {
    Allow,
    Deny { reason: DenyReason },
}

pub fn decide(req: &DiscardRequest) -> DiscardVerdict {
    let taint = match req {
        DiscardRequest::Unplanned { .. } => {
            return DiscardVerdict::Deny {
                reason: DenyReason::NoRestoration,
            };
        }
        DiscardRequest::Planned(discard) => discard.taint(),
    };
    if taint.is_empty() {
        DiscardVerdict::Allow
    } else {
        DiscardVerdict::Deny {
            reason: DenyReason::Tainted,
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
    use super::super::request::{Discard, DiscardRequest, Restoration};
    use super::*;
    use crate::address::Address;
    use crate::budget::ByteLen;
    use crate::locator::Locator;
    use crate::taint::{TaintSet, TaintSource};
    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }
    fn tracked() -> Restoration {
        Restoration::Tracked(Locator::parse(&format!("file:b/x.md@{}", "ab".repeat(20))).unwrap())
    }
    fn clean(paths: Vec<Address>, bytes: u64) -> Discard {
        Discard::new(paths, tracked(), TaintSet::empty(), ByteLen::new(bytes)).unwrap()
    }

    #[test]
    fn the_decision_table_holds_in_order() {

        assert_eq!(
            decide(&DiscardRequest::Planned(clean(vec![addr("b/x.md")], 100))),
            DiscardVerdict::Allow
        );
        let many: Vec<Address> = (0..17).map(|i| addr(&format!("b/f{i}.md"))).collect();
        assert_eq!(
            decide(&DiscardRequest::Planned(clean(many, 4_000_000))),
            DiscardVerdict::Allow,
            "a big delete that can be put back is a delete"
        );

        let tainted = Discard::new(
            vec![addr("b/x.md")],
            tracked(),
            TaintSet::of(TaintSource::new("web:evil").unwrap()),
            ByteLen::new(1),
        )
        .unwrap();
        assert_eq!(
            decide(&DiscardRequest::Planned(tainted)),
            DiscardVerdict::Deny {
                reason: DenyReason::Tainted
            }
        );

        assert_eq!(
            decide(&DiscardRequest::Unplanned {
                paths: vec![addr("b/x.md")],
                taint: TaintSet::empty(),
                total_bytes: ByteLen::new(1)
            }),
            DiscardVerdict::Deny {
                reason: DenyReason::NoRestoration
            }
        );
    }
}

