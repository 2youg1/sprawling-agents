// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegateKind {
    Resident,
    Ephemeral,
}

impl DelegateKind {

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DelegateKind::Resident => "resident",
            DelegateKind::Ephemeral => "ephemeral",
        }
    }
}

#[derive(Debug)]
pub struct Delegator(());

impl Delegator {

    pub fn root() -> Delegator {
        Delegator(())
    }

    pub fn delegate(&self, kind: DelegateKind) -> Delegate {
        Delegate { kind }
    }
}

#[derive(Debug)]
pub struct Delegate {
    kind: DelegateKind,
}

impl Delegate {
    pub fn kind(&self) -> &DelegateKind {
        &self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Root,
    Delegated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegationVerdict {
    Allow,
    Deny,
}

pub fn admit(parent: Depth, kind: &DelegateKind) -> DelegationVerdict {
    match parent {
        Depth::Root => {

            match kind {
                DelegateKind::Resident | DelegateKind::Ephemeral => DelegationVerdict::Allow,
            }
        }
        Depth::Delegated => DelegationVerdict::Deny,
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

    #[test]
    fn root_spawns_both_kinds_delegated_spawns_none() {
        assert_eq!(
            admit(Depth::Root, &DelegateKind::Resident),
            DelegationVerdict::Allow
        );
        assert_eq!(
            admit(Depth::Root, &DelegateKind::Ephemeral),
            DelegationVerdict::Allow
        );
        assert_eq!(
            admit(Depth::Delegated, &DelegateKind::Resident),
            DelegationVerdict::Deny
        );
        assert_eq!(
            admit(Depth::Delegated, &DelegateKind::Ephemeral),
            DelegationVerdict::Deny
        );
    }

    #[test]
    fn the_static_path_mints_one_level_only() {
        let root = Delegator::root();
        let child = root.delegate(DelegateKind::Ephemeral);
        assert_eq!(child.kind(), &DelegateKind::Ephemeral);

    }
}
