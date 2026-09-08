// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Event kinds: the closed set and its window classes.

use serde::{Deserialize, Serialize};

/// The closed event vocabulary.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum EventKind {
    // Genesis and space (2).
    CityInitialized,
    BuildingCreated,
    // Base set (18).
    RunStarted,
    RunForked,
    PromptAssembled,
    ModelCalled,
    ModelReturned,
    ToolCalled,
    ToolResult,
    ResultOffloaded,
    GateChecked,
    GateDenied,
    CheckpointCommitted,
    HandoffWritten,
    SteerReceived,
    CancelReceived,
    WatchdogFired,
    BudgetLimit,
    RunFrozen,
    LogTruncated,
    // Collaboration (19).
    SignalEnqueued,
    SignalConsumed,
    DraftHeld,
    DraftResolved,
    GoalRegistered,
    GoalConflict,
    ArbitrationVerdict,
    RepairStarted,
    RepairReused,
    WorktreeOpened,
    PrOpened,
    PrMerged,
    PrRejected,
    RoadmapClaimed,
    RoadmapFinished,
    RoadmapReleased,
    /// A branch was divided into children. The plan grew rather than
    /// moved, which is a different fact from any row changing state and
    /// is the only one that changes what the denominator is made of.
    RoadmapSplit,
    /// A node cannot proceed, and why. Distinct from `roadmap_released`:
    /// a released node is back in the ready set, a blocked one is red
    /// and everything behind it is waiting.
    RoadmapBlocked,
    /// A city was given a goal to keep working towards, or that goal
    /// was paused, resumed or cleared. Recorded because it is the one
    /// thing in the city that starts work without a person asking, and
    /// "who set this running" has to be answerable afterwards.
    PursuitChanged,
    // Governance and facilities (17).
    ApprovalRequested,
    ApprovalResolved,
    PolicyCreated,
    PolicyRevoked,
    TaintPromoted,
    CrossBuildingTransfer,
    TakeoverStarted,
    RollbackApplied,
    CityHalted,
    BackpressureShed,
    DigestInvalidated,
    EndpointAttached,
    /// What a base URL says it serves, asked before anything is
    /// attached. A separate fact from `EndpointAttached`: a person may
    /// look at what a key buys and register none of it.
    EndpointProbed,
    EndpointLost,
    ModelSelected,
    ProviderDegraded,
    LoginStarted,
    EvalRun,
    AssetArchived,
    CredentialLent,
    // Privacy and Discard (5).
    SecretCaptured,
    SecretEgressBlocked,
    FileDiscarded,
    DiscardRestored,
    AutonomyChanged,
}

/// The two-way partition; the sole criterion is "does the payload decide
/// model-request bytes" (C16). There is no third class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowClass {
    InWindow,
    RecordOnly,
}

impl EventKind {
    /// Every kind, in the order the SPEC table lists them. Data face for counting tests
    /// and (from S2 on) `xtask specalign`.
    pub const ALL: [EventKind; 64] = [
        EventKind::CityInitialized,
        EventKind::BuildingCreated,
        EventKind::RunStarted,
        EventKind::RunForked,
        EventKind::PromptAssembled,
        EventKind::ModelCalled,
        EventKind::ModelReturned,
        EventKind::ToolCalled,
        EventKind::ToolResult,
        EventKind::ResultOffloaded,
        EventKind::GateChecked,
        EventKind::GateDenied,
        EventKind::CheckpointCommitted,
        EventKind::HandoffWritten,
        EventKind::SteerReceived,
        EventKind::CancelReceived,
        EventKind::WatchdogFired,
        EventKind::BudgetLimit,
        EventKind::RunFrozen,
        EventKind::LogTruncated,
        EventKind::SignalEnqueued,
        EventKind::SignalConsumed,
        EventKind::DraftHeld,
        EventKind::DraftResolved,
        EventKind::GoalRegistered,
        EventKind::GoalConflict,
        EventKind::ArbitrationVerdict,
        EventKind::RepairStarted,
        EventKind::RepairReused,
        EventKind::WorktreeOpened,
        EventKind::PrOpened,
        EventKind::PrMerged,
        EventKind::PrRejected,
        EventKind::RoadmapClaimed,
        EventKind::RoadmapFinished,
        EventKind::RoadmapReleased,
        EventKind::RoadmapSplit,
        EventKind::RoadmapBlocked,
        EventKind::PursuitChanged,
        EventKind::ApprovalRequested,
        EventKind::ApprovalResolved,
        EventKind::PolicyCreated,
        EventKind::PolicyRevoked,
        EventKind::TaintPromoted,
        EventKind::CrossBuildingTransfer,
        EventKind::TakeoverStarted,
        EventKind::RollbackApplied,
        EventKind::CityHalted,
        EventKind::BackpressureShed,
        EventKind::DigestInvalidated,
        EventKind::EndpointAttached,
        EventKind::EndpointProbed,
        EventKind::EndpointLost,
        EventKind::ModelSelected,
        EventKind::ProviderDegraded,
        EventKind::LoginStarted,
        EventKind::EvalRun,
        EventKind::AssetArchived,
        EventKind::CredentialLent,
        EventKind::SecretCaptured,
        EventKind::SecretEgressBlocked,
        EventKind::FileDiscarded,
        EventKind::DiscardRestored,
        EventKind::AutonomyChanged,
    ];

    /// The partition authority. Exhaustive on purpose: adding a variant
    /// without deciding its class is a compile error, not a default.
    pub fn window_class(&self) -> WindowClass {
        match self {
            EventKind::PromptAssembled
            | EventKind::ModelCalled
            | EventKind::ModelReturned
            | EventKind::ToolCalled
            | EventKind::ToolResult
            | EventKind::ResultOffloaded
            | EventKind::SteerReceived
            | EventKind::SignalConsumed => WindowClass::InWindow,
            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::RunStarted
            | EventKind::RunForked
            | EventKind::GateChecked
            | EventKind::GateDenied
            | EventKind::CheckpointCommitted
            | EventKind::HandoffWritten
            | EventKind::CancelReceived
            | EventKind::WatchdogFired
            | EventKind::BudgetLimit
            | EventKind::RunFrozen
            | EventKind::LogTruncated
            | EventKind::SignalEnqueued
            | EventKind::DraftHeld
            | EventKind::DraftResolved
            | EventKind::GoalRegistered
            | EventKind::GoalConflict
            | EventKind::ArbitrationVerdict
            | EventKind::RepairStarted
            | EventKind::RepairReused
            | EventKind::WorktreeOpened
            | EventKind::PrOpened
            | EventKind::PrMerged
            | EventKind::PrRejected
            | EventKind::RoadmapClaimed
            | EventKind::RoadmapFinished
            | EventKind::RoadmapReleased
            | EventKind::RoadmapSplit
            | EventKind::RoadmapBlocked
            | EventKind::PursuitChanged
            | EventKind::ApprovalRequested
            | EventKind::ApprovalResolved
            | EventKind::PolicyCreated
            | EventKind::PolicyRevoked
            | EventKind::TaintPromoted
            | EventKind::CrossBuildingTransfer
            | EventKind::TakeoverStarted
            | EventKind::RollbackApplied
            | EventKind::CityHalted
            | EventKind::BackpressureShed
            | EventKind::DigestInvalidated
            | EventKind::EndpointAttached
            | EventKind::EndpointProbed
            | EventKind::EndpointLost
            | EventKind::ModelSelected
            | EventKind::ProviderDegraded
            | EventKind::LoginStarted
            | EventKind::EvalRun
            | EventKind::AssetArchived
            | EventKind::CredentialLent
            | EventKind::SecretCaptured
            | EventKind::SecretEgressBlocked
            | EventKind::FileDiscarded
            | EventKind::DiscardRestored
            | EventKind::AutonomyChanged => WindowClass::RecordOnly,
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
    use crate::error::{AxCode, Carrier};
    use std::collections::BTreeSet;
    #[test]
    fn event_kind_is_61_with_exactly_8_in_window() {
        assert_eq!(EventKind::ALL.len(), 64);
        let names: BTreeSet<String> = EventKind::ALL
            .iter()
            .map(|k| serde_json::to_string(k).unwrap())
            .collect();
        assert_eq!(names.len(), 64, "serde spellings must be unique");
        let in_window: Vec<EventKind> = EventKind::ALL
            .into_iter()
            .filter(|k| k.window_class() == WindowClass::InWindow)
            .collect();
        assert_eq!(in_window.len(), 8);
        for k in [
            EventKind::PromptAssembled,
            EventKind::ModelCalled,
            EventKind::ModelReturned,
            EventKind::ToolCalled,
            EventKind::ToolResult,
            EventKind::ResultOffloaded,
            EventKind::SteerReceived,
            EventKind::SignalConsumed,
        ] {
            assert_eq!(k.window_class(), WindowClass::InWindow);
        }
        assert_eq!(
            serde_json::to_string(&EventKind::CityInitialized).unwrap(),
            "\"city_initialized\""
        );
    }

    #[test]
    fn carrier_declarations_cover_all_35_codes() {
        let mut loadtime = 0;
        let mut gate = 0;
        let mut tool = 0;
        for code in AxCode::ALL {
            match code.carrier() {
                Carrier::Loadtime => loadtime += 1,
                Carrier::Event(EventKind::GateDenied) => gate += 1,
                Carrier::Event(EventKind::ToolResult) => tool += 1,
                Carrier::Event(_) => {}
            }
        }
        assert_eq!(loadtime, 5, "loadtime whitelist is closed at five");
        assert_eq!(gate, 7);
        assert_eq!(tool, 18);
        assert_eq!(
            AxCode::BudgetExhausted.carrier(),
            Carrier::Event(EventKind::BudgetLimit)
        );
        assert_eq!(
            AxCode::ApprovalPending.carrier(),
            Carrier::Event(EventKind::ApprovalRequested)
        );
        assert_eq!(
            AxCode::ApprovalDenied.carrier(),
            Carrier::Event(EventKind::ApprovalResolved)
        );
        assert_eq!(
            AxCode::Provider.carrier(),
            Carrier::Event(EventKind::ProviderDegraded)
        );
        assert_eq!(
            AxCode::EndpointDialectUnsupported.carrier(),
            Carrier::Event(EventKind::EndpointLost)
        );
        assert_eq!(
            AxCode::LoopSuspected.carrier(),
            Carrier::Event(EventKind::WatchdogFired)
        );
    }
}
