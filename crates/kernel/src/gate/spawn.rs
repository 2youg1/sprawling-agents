// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The spawn admission: who may hand work down, and how far.
//!
//! Handing work to a second agent needs no person's answer: the depth
//! rule bounds it, one level and no further, and a rule the type system
//! already carries is not a question.

use crate::delegation::{self, DelegateKind, DelegationVerdict, Depth};
use crate::error::{AxCode, AxError, GateRefusal};

use super::GateOutcome;

/// Delegates do not delegate (10.1). The refusal teaches the
/// alternative instead of hiding the tool.
pub fn spawn(parent: Depth, kind: &DelegateKind) -> GateOutcome {
    match delegation::admit(parent, kind) {
        DelegationVerdict::Allow => GateOutcome::Allow,
        DelegationVerdict::Deny => GateOutcome::Deny {
            refusal: Box::new(
                AxError::refusal(
                    AxCode::DelegationDepth,
                    "spawn delegate",
                    format!("{kind:?}"),
                    GateRefusal::new(
                        "delegates do not delegate: one level deep",
                        "a delegated position requested a spawn",
                        "return this subtask to the resident who spawned you; \
                         that resident can delegate it",
                    ),
                )
                .with_recovery(
                    "finish this subtask yourself, or end the turn with what you have so \
                     the resident who spawned you can delegate the rest",
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
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn the_spawn_door_teaches_the_way_back_up() {
        assert!(matches!(
            spawn(Depth::Root, &DelegateKind::Ephemeral),
            GateOutcome::Allow
        ));
        let GateOutcome::Deny { refusal } = spawn(Depth::Delegated, &DelegateKind::Ephemeral)
        else {
            panic!("a delegate may not delegate")
        };
        assert_eq!(refusal.code(), &AxCode::DelegationDepth);
        let parts = refusal.gate().expect("gate refusals carry three parts");
        assert!(!parts.rule().is_empty());
        assert!(!parts.violation().is_empty());
        assert!(parts.alternative().contains("spawned you"));
    }
}
