// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The discard door: whether these files may go.
//!
//! The decision is `crate::discard`'s; this file shapes the refusal the
//! model reads, which is the only thing a door adds to a verdict.

use crate::discard::{DenyReason, DiscardRequest, DiscardVerdict, decide};
use crate::error::{AxCode, AxError, GateRefusal};

use super::GateOutcome;

/// Judges one discard request and teaches the way out when it refuses.
pub fn discard(req: &DiscardRequest, action_desc: &str) -> GateOutcome {
    let DiscardVerdict::Deny { reason } = decide(req) else {
        return GateOutcome::Allow;
    };
    let (code, parts, recovery) = match reason {
        DenyReason::NoRestoration => (
            AxCode::DiscardIrreversible,
            GateRefusal::new(
                "every discard carries a resolvable restoration (C14)",
                "this request names no restoration plan",
                "inter the originals in CAS (Interred) and retry with that locator, \
                 or cite the `file:` locator of the tracked copy",
            ),
            "call `discard` again with a restoration: a `file:` locator for a tracked \
             file, or a `cas:` locator from interring the originals first",
        ),
        DenyReason::Tainted => (
            AxCode::TaintedAction,
            GateRefusal::new(
                "an effect a run derived from outside content is refused (C15)",
                "this request carries taint, so something outside the city asked for it",
                "delete nothing on behalf of fetched content; if the deletion is your \
                 own conclusion, state it in a run that did not read that content",
            ),
            "re-raise the discard from work that did not start outside the city, or \
             add the source to `[sandbox] trusted` in `CONFIG.toml` if this city trusts it",
        ),
    };
    GateOutcome::Deny {
        refusal: Box::new(
            AxError::refusal(code, "discard files", action_desc, parts).with_recovery(recovery),
        ),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::address::Address;
    use crate::budget::ByteLen;
    use crate::discard::{Discard, Restoration};
    use crate::locator::Locator;
    use crate::taint::{TaintSet, TaintSource};

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    fn tracked() -> Restoration {
        Restoration::Tracked(Locator::parse(&format!("file:b/x.md@{}", "ab".repeat(20))).unwrap())
    }

    fn refusal_of(outcome: GateOutcome) -> AxError {
        match outcome {
            GateOutcome::Deny { refusal } => *refusal,
            GateOutcome::Allow => panic!("expected a refusal"),
        }
    }

    #[test]
    fn a_delete_with_no_way_back_is_refused_with_the_way_to_make_one() {
        let unplanned = DiscardRequest::Unplanned {
            paths: vec![addr("b/x.md")],
            taint: TaintSet::empty(),
            total_bytes: ByteLen::new(10),
        };
        let refusal = refusal_of(discard(&unplanned, "delete b/x.md"));
        assert_eq!(refusal.code(), &AxCode::DiscardIrreversible);
        assert!(refusal.gate().unwrap().alternative().contains("Interred"));
    }

    #[test]
    fn a_delete_asked_for_by_outside_content_is_refused() {
        let tainted = Discard::new(
            vec![addr("b/x.md")],
            tracked(),
            TaintSet::of(TaintSource::new("web:evil").unwrap()),
            ByteLen::new(1),
        )
        .unwrap();
        let refusal = refusal_of(discard(&DiscardRequest::Planned(tainted), "delete b/x.md"));
        assert_eq!(refusal.code(), &AxCode::TaintedAction);
        assert!(refusal.gate().unwrap().violation().contains("taint"));
    }

    #[test]
    fn a_planned_clean_delete_passes_at_any_scale() {
        let many: Vec<Address> = (0..40).map(|i| addr(&format!("b/f{i}.md"))).collect();
        let planned =
            Discard::new(many, tracked(), TaintSet::empty(), ByteLen::new(9_000_000)).unwrap();
        assert!(matches!(
            discard(&DiscardRequest::Planned(planned), "clear the build tree"),
            GateOutcome::Allow
        ));
    }
}
