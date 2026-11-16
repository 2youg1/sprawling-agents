// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::completion::{CITABLE, citable_words};
use crate::error::{AxCode, AxError};
use crate::event::EventRef;
use crate::locator::Locator;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ResidentId(String);

impl ResidentId {
    pub fn new(raw: impl Into<String>) -> Option<ResidentId> {
        let raw = raw.into();
        if raw.is_empty() {
            None
        } else {
            Some(ResidentId(raw))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub locator: Locator,
    pub by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    locator: Locator,
    verified_by: EventRef,
}

impl Artifact {

    pub fn verify(claim: Claim, evidence: EventRef) -> Result<Artifact, AxError> {
        if CITABLE.contains(&evidence.kind()) {
            return Ok(Artifact {
                locator: claim.locator,
                verified_by: evidence,
            });
        }
        Err(AxError::failure(
            AxCode::EvidenceMissing,
            "verify claim",
            claim.locator.to_string(),
        )
        .with_recovery(format!(
            "evidence must be a {} ref, got {:?}; run the verification and cite its result",
            citable_words()?,
            evidence.kind()
        )))
    }

    pub fn locator(&self) -> &Locator {
        &self.locator
    }

    pub fn verified_by(&self) -> &EventRef {
        &self.verified_by
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterVerdict {
    Registered,
    AlreadyRegistered,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    artifacts: BTreeMap<String, Artifact>,
    assets: BTreeSet<String>,
    residents: BTreeSet<ResidentId>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    pub fn register_artifact(&mut self, artifact: Artifact) -> RegisterVerdict {
        let key = artifact.locator.to_string();
        match self.artifacts.entry(key) {
            std::collections::btree_map::Entry::Occupied(_) => RegisterVerdict::AlreadyRegistered,
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(artifact);
                RegisterVerdict::Registered
            }
        }
    }

    pub fn promote_asset(&mut self, locator: &Locator) -> Result<RegisterVerdict, AxError> {
        let key = locator.to_string();
        if !self.artifacts.contains_key(&key) {
            return Err(AxError::failure(AxCode::PathNotFound, "promote asset", key)
                .with_nearby(self.artifacts.keys().cloned().collect())
                .with_recovery("register the artifact first, then promote it"));
        }
        if self.assets.insert(key) {
            Ok(RegisterVerdict::Registered)
        } else {
            Ok(RegisterVerdict::AlreadyRegistered)
        }
    }

    pub fn register_resident(&mut self, id: ResidentId) -> RegisterVerdict {
        if self.residents.insert(id) {
            RegisterVerdict::Registered
        } else {
            RegisterVerdict::AlreadyRegistered
        }
    }

    pub fn artifact(&self, locator: &Locator) -> Option<&Artifact> {
        self.artifacts.get(&locator.to_string())
    }

    pub fn is_asset(&self, locator: &Locator) -> bool {
        self.assets.contains(&locator.to_string())
    }

    pub fn is_asset_at(&self, addr: &crate::address::Address) -> bool {
        self.assets.iter().any(|key| {
            matches!(
                Locator::parse(key),
                Ok(Locator::File { address, .. }) if address == *addr
            )
        })
    }

    pub fn is_resident(&self, id: &ResidentId) -> bool {
        self.residents.contains(id)
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
    use crate::event::{EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
    use crate::ledger::GENESIS_PREV;

    fn evidence(kind: EventKind) -> EventRef {
        let draft = EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(0),
            who: "city".into(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        };
        EventRecord::from_draft(draft, Seq::FIRST, GENESIS_PREV).to_ref()
    }

    fn locator() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "cd".repeat(32))).unwrap()
    }

    fn claim() -> Claim {
        Claim {
            locator: locator(),
            by: "worker@sim.1".into(),
        }
    }

    #[test]
    fn verification_accepts_only_citable_kinds() {
        assert!(Artifact::verify(claim(), evidence(EventKind::ToolResult)).is_ok());
        assert!(Artifact::verify(claim(), evidence(EventKind::ModelReturned)).is_ok());
        let err = Artifact::verify(claim(), evidence(EventKind::RunStarted)).unwrap_err();
        assert_eq!(err.code(), &AxCode::EvidenceMissing);
    }

    #[test]
    fn a_harness_answer_verifies_a_claim() {
        let verified = Artifact::verify(claim(), evidence(EventKind::HarnessAnswered));
        assert!(verified.is_ok(), "{verified:?}");
        assert_eq!(
            verified.unwrap().verified_by().kind(),
            EventKind::HarnessAnswered
        );
    }

    #[test]
    fn promotion_requires_prior_registration() {
        let mut registry = Registry::new();
        let err = registry.promote_asset(&locator()).unwrap_err();
        assert_eq!(err.code(), &AxCode::PathNotFound);
        let artifact = Artifact::verify(claim(), evidence(EventKind::ToolResult)).unwrap();
        assert_eq!(
            registry.register_artifact(artifact.clone()),
            RegisterVerdict::Registered
        );
        assert_eq!(
            registry.register_artifact(artifact),
            RegisterVerdict::AlreadyRegistered
        );
        assert_eq!(
            registry.promote_asset(&locator()).unwrap(),
            RegisterVerdict::Registered
        );
        assert!(registry.is_asset(&locator()));
    }

    #[test]
    fn each_query_answers_for_what_the_registry_holds_and_nothing_else() {
        let held = Locator::parse(&format!("file:lab/notes.md@{}", "ab".repeat(20))).unwrap();
        let place = crate::address::Address::parse("lab/notes.md").unwrap();
        let elsewhere = crate::address::Address::parse("lab/other.md").unwrap();
        let resident = ResidentId::new("worker@sim.1").unwrap();
        let mut registry = Registry::new();
        let ask = |registry: &Registry| {
            (
                registry.artifact(&held).cloned(),
                registry.is_asset(&held),
                registry.is_asset_at(&place),
                registry.is_asset_at(&elsewhere),
                registry.is_resident(&resident),
            )
        };
        assert_eq!(ask(&registry), (None, false, false, false, false));
        let artifact = Artifact::verify(
            Claim {
                locator: held.clone(),
                by: "worker@sim.1".into(),
            },
            evidence(EventKind::ToolResult),
        )
        .unwrap();
        registry.register_artifact(artifact.clone());
        assert_eq!(
            ask(&registry),
            (Some(artifact.clone()), false, false, false, false)
        );
        registry.promote_asset(&held).unwrap();
        assert_eq!(
            ask(&registry),
            (Some(artifact.clone()), true, true, false, false)
        );
        registry.register_resident(ResidentId::new("someone@else").unwrap());
        assert_eq!(ask(&registry), (Some(artifact), true, true, false, false));
    }

    #[test]
    fn the_census_deduplicates() {
        let mut registry = Registry::new();
        let id = ResidentId::new("worker@sim.1").unwrap();
        assert_eq!(
            registry.register_resident(id.clone()),
            RegisterVerdict::Registered
        );
        assert_eq!(
            registry.register_resident(id.clone()),
            RegisterVerdict::AlreadyRegistered
        );
        assert!(registry.is_resident(&id));
    }
}
