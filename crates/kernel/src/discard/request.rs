// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Discard requests: restoration routes and the planned/unplanned shapes.

use serde::{Deserialize, Serialize};

use crate::address::Address;
use crate::budget::ByteLen;
use crate::error::{AxCode, AxError};
use crate::locator::Locator;
use crate::taint::TaintSet;

/// Tracked rides git (`file:`), Interred rides CAS (`cas:`), Rebuildable
/// names its reason. No fourth storage authority exists.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Restoration {
    Tracked(Locator),
    Interred(Locator),
    Rebuildable { reason: String },
}

/// A planned discard. Private fields; the sole constructor checks the
/// plan's scheme — C14's type half.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discard {
    paths: Vec<Address>,
    plan: Restoration,
    taint: TaintSet,
    total_bytes: ByteLen,
}

impl Discard {
    /// Restoration is mandatory and scheme-checked: Tracked wants
    /// `file:`, Interred wants `cas:`, Rebuildable wants a non-empty
    /// reason — violations are `E_DISCARD_IRREVERSIBLE` (an unparsable
    /// plan is no plan). Empty paths are `E_INVALID_ARGS`.
    pub fn new(
        paths: Vec<Address>,
        plan: Restoration,
        taint: TaintSet,
        total_bytes: ByteLen,
    ) -> Result<Discard, AxError> {
        if paths.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "construct discard",
                "empty path list",
            ));
        }
        let plan_ok = match &plan {
            Restoration::Tracked(locator) => matches!(locator, Locator::File { .. }),
            Restoration::Interred(locator) => matches!(locator, Locator::Cas { .. }),
            Restoration::Rebuildable { reason } => !reason.is_empty(),
        };
        if !plan_ok {
            return Err(AxError::failure(
                AxCode::DiscardIrreversible,
                "construct discard",
                "restoration plan does not resolve",
            )
            .with_recovery(
                "Tracked cites a file: locator, Interred a cas: locator, \
                 Rebuildable a non-empty reason",
            ));
        }
        Ok(Discard {
            paths,
            plan,
            taint,
            total_bytes,
        })
    }

    pub fn paths(&self) -> &[Address] {
        &self.paths
    }

    pub fn plan(&self) -> &Restoration {
        &self.plan
    }

    pub fn taint(&self) -> &TaintSet {
        &self.taint
    }

    pub fn total_bytes(&self) -> ByteLen {
        self.total_bytes
    }
}

/// What reaches the door: a planned discard, or an unplanned request
/// from the exec forecast path (text prediction cannot mint plans).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscardRequest {
    Planned(Discard),
    Unplanned {
        paths: Vec<Address>,
        taint: TaintSet,
        total_bytes: ByteLen,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalateReason {
    /// Tainted discards never auto-pass, whatever the size (C15).
    Tainted,
    FilesOverMax,
    BytesOverMax,
    RegistryAsset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    NoRestoration,
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
    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }
    fn tracked() -> Restoration {
        Restoration::Tracked(Locator::parse(&format!("file:b/x.md@{}", "ab".repeat(20))).unwrap())
    }
    fn interred() -> Restoration {
        Restoration::Interred(Locator::parse(&format!("cas:b3-{}", "cd".repeat(32))).unwrap())
    }
    #[test]
    fn restoration_schemes_are_checked_at_construction() {
        assert!(
            Discard::new(
                vec![addr("b/x.md")],
                tracked(),
                TaintSet::empty(),
                ByteLen::new(1)
            )
            .is_ok()
        );
        assert!(
            Discard::new(
                vec![addr("b/x.md")],
                interred(),
                TaintSet::empty(),
                ByteLen::new(1)
            )
            .is_ok()
        );
        // Tracked with a cas: locator is an unresolvable plan.
        let wrong =
            Restoration::Tracked(Locator::parse(&format!("cas:b3-{}", "cd".repeat(32))).unwrap());
        let err = Discard::new(
            vec![addr("b/x.md")],
            wrong,
            TaintSet::empty(),
            ByteLen::new(1),
        )
        .unwrap_err();
        assert_eq!(err.code(), &AxCode::DiscardIrreversible);
        let empty_reason = Restoration::Rebuildable {
            reason: String::new(),
        };
        assert!(
            Discard::new(
                vec![addr("b/x.md")],
                empty_reason,
                TaintSet::empty(),
                ByteLen::new(1)
            )
            .is_err()
        );
        let no_paths = Discard::new(vec![], tracked(), TaintSet::empty(), ByteLen::new(1));
        assert_eq!(no_paths.unwrap_err().code(), &AxCode::InvalidArgs);
    }
}
