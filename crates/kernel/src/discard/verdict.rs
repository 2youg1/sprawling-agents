// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Discard verdicts: the decision table.

use crate::consts_policy::{DISCARD_BYTES_MAX, DISCARD_FILES_MAX};
use crate::registry::Registry;

use super::request::{DenyReason, DiscardRequest, EscalateReason};

/// Deliberately exhaustive verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardVerdict {
    Allow,
    Escalate { reason: EscalateReason },
    Deny { reason: DenyReason },
}

/// The decision table (7.2), in fixed order: no plan denies; taint
/// escalates always; then scale (files, bytes); then registry assets;
/// the rest passes — a restorable, small, clean-handed delete is not
/// worth an interruption.
pub fn decide(req: &DiscardRequest, registry: &Registry) -> DiscardVerdict {
    let (paths, taint, total_bytes) = match req {
        DiscardRequest::Unplanned { .. } => {
            return DiscardVerdict::Deny {
                reason: DenyReason::NoRestoration,
            };
        }
        DiscardRequest::Planned(discard) => {
            (discard.paths(), discard.taint(), discard.total_bytes())
        }
    };
    if !taint.is_empty() {
        return DiscardVerdict::Escalate {
            reason: EscalateReason::Tainted,
        };
    }
    let over_files = u32::try_from(paths.len()).map_or(true, |count| count > DISCARD_FILES_MAX);
    if over_files {
        return DiscardVerdict::Escalate {
            reason: EscalateReason::FilesOverMax,
        };
    }
    if total_bytes.get() > DISCARD_BYTES_MAX {
        return DiscardVerdict::Escalate {
            reason: EscalateReason::BytesOverMax,
        };
    }
    if paths.iter().any(|p| registry.is_asset_at(p)) {
        return DiscardVerdict::Escalate {
            reason: EscalateReason::RegistryAsset,
        };
    }
    DiscardVerdict::Allow
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
        let registry = Registry::new();
        // Allow: small, clean, planned.
        assert_eq!(
            decide(
                &DiscardRequest::Planned(clean(vec![addr("b/x.md")], 100)),
                &registry
            ),
            DiscardVerdict::Allow
        );
        // Tainted first, regardless of scale.
        let tainted = Discard::new(
            vec![addr("b/x.md")],
            tracked(),
            TaintSet::of(TaintSource::new("web:evil").unwrap()),
            ByteLen::new(1),
        )
        .unwrap();
        assert_eq!(
            decide(&DiscardRequest::Planned(tainted), &registry),
            DiscardVerdict::Escalate {
                reason: EscalateReason::Tainted
            }
        );
        // Files over max.
        let many: Vec<Address> = (0..17).map(|i| addr(&format!("b/f{i}.md"))).collect();
        assert_eq!(
            decide(&DiscardRequest::Planned(clean(many, 1)), &registry),
            DiscardVerdict::Escalate {
                reason: EscalateReason::FilesOverMax
            }
        );
        // Bytes over max.
        assert_eq!(
            decide(
                &DiscardRequest::Planned(clean(vec![addr("b/big.bin")], 1_048_577)),
                &registry
            ),
            DiscardVerdict::Escalate {
                reason: EscalateReason::BytesOverMax
            }
        );
        // Unplanned denies.
        assert_eq!(
            decide(
                &DiscardRequest::Unplanned {
                    paths: vec![addr("b/x.md")],
                    taint: TaintSet::empty(),
                    total_bytes: ByteLen::new(1)
                },
                &registry
            ),
            DiscardVerdict::Deny {
                reason: DenyReason::NoRestoration
            }
        );
    }

    #[test]
    fn registry_assets_escalate() {
        use crate::event::{EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
        use crate::ledger::GENESIS_PREV;
        use crate::registry::{Artifact, Claim};
        let mut registry = Registry::new();
        let locator = Locator::parse(&format!("file:b/asset.md@{}", "ab".repeat(20))).unwrap();
        let evidence = EventRecord::from_draft(
            EventDraft {
                run: RunId::CITY,
                t: TimeMs::new(0),
                who: "city".into(),
                addr: None,
                kind: EventKind::ToolResult,
                data: Payload::empty(),
                ig: false,
            },
            Seq::FIRST,
            GENESIS_PREV,
        )
        .to_ref();
        let artifact = Artifact::verify(
            Claim {
                locator: locator.clone(),
                by: "worker".into(),
            },
            evidence,
        )
        .unwrap();
        registry.register_artifact(artifact);
        registry.promote_asset(&locator).unwrap();
        assert_eq!(
            decide(
                &DiscardRequest::Planned(clean(vec![addr("b/asset.md")], 10)),
                &registry
            ),
            DiscardVerdict::Escalate {
                reason: EscalateReason::RegistryAsset
            }
        );
    }
}

#[cfg(kani)]
mod verification {
    //! V5: the fifth door fails closed — no plan never allows, taint
    //! never allows.

    use super::*;
    use crate::taint::TaintSource;

    #[kani::proof]
    fn unplanned_never_allows() {
        let registry = Registry::new();
        let req = DiscardRequest::Unplanned {
            paths: vec![Address::parse("b/x").unwrap()],
            taint: TaintSet::empty(),
            total_bytes: ByteLen::new(kani::any()),
        };
        assert!(matches!(
            decide(&req, &registry),
            DiscardVerdict::Deny { .. }
        ));
    }

    #[kani::proof]
    fn tainted_never_allows() {
        let registry = Registry::new();
        let source = TaintSource::new("web:x").unwrap();
        let discard = Discard::new(
            vec![Address::parse("b/x").unwrap()],
            Restoration::Rebuildable {
                reason: "cargo target".to_owned(),
            },
            TaintSet::of(source),
            ByteLen::new(kani::any()),
        )
        .unwrap();
        assert!(matches!(
            decide(&DiscardRequest::Planned(discard), &registry),
            DiscardVerdict::Escalate {
                reason: EscalateReason::Tainted
            }
        ));
    }
}
