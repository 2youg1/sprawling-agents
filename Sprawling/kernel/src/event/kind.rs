// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

mod window;
pub use window::WindowClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum EventKind {

    CityInitialized,
    BuildingCreated,

    BuildingConfigured,

    BuildingRemoved,

    SessionOpened,
    RunStarted,
    RunForked,
    PromptAssembled,

    PromptShapeCompared,
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

    RoadmapSplit,

    RoadmapBlocked,

    PursuitChanged,

    ApprovalRequested,
    ApprovalResolved,
    PolicyCreated,
    PolicyRevoked,
    TaintPromoted,
    CrossBuildingTransfer,
    CityHalted,
    BackpressureShed,
    DigestInvalidated,
    EndpointAttached,

    EndpointProbed,
    EndpointLost,
    ModelSelected,
    ProviderDegraded,

    LoginStarted,
    EvalRun,
    AssetArchived,
    CredentialLent,

    SecretCaptured,
    SecretEgressBlocked,
    FileDiscarded,
    DiscardRestored,
    AutonomyChanged,

    WentBack,
    FileRestored,

    GovernedDocumentWritten,

    SpineDocumentWritten,

    RulesChanged,

    ToolkitLinkOpened,

    EmbeddingCalled,

    RerankCalled,

    AdviserAsked,

    AdviserAnswered,

    AdviserFellBack,

    CacheRenewed,

    HarnessReported,

    HarnessAnswered,

    RemoteOpened,

    RemoteClosed,

    DevicePaired,

    DeviceRevoked,

    RemoteSessionStarted,

    DocumentWritten,

    ProposalOffered,

    ProposalDecided,

    ProposalWithdrawn,

    RunPolicyChanged,

    SessionNamed,

    SkillAudited,

    SignalWaitStarted,

    SignalWaitEnded,

    SignalLanded,

    SkillShelved,

    PrefixHeld,
}

impl EventKind {

    pub const ALL: [EventKind; 97] = [
        EventKind::CityInitialized,
        EventKind::BuildingCreated,
        EventKind::BuildingConfigured,
        EventKind::BuildingRemoved,
        EventKind::SessionOpened,
        EventKind::RunStarted,
        EventKind::RunForked,
        EventKind::PromptAssembled,
        EventKind::PromptShapeCompared,
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
        EventKind::WentBack,
        EventKind::FileRestored,
        EventKind::GovernedDocumentWritten,
        EventKind::SpineDocumentWritten,
        EventKind::RulesChanged,
        EventKind::ToolkitLinkOpened,
        EventKind::EmbeddingCalled,
        EventKind::RerankCalled,
        EventKind::AdviserAsked,
        EventKind::AdviserAnswered,
        EventKind::AdviserFellBack,
        EventKind::CacheRenewed,
        EventKind::HarnessReported,
        EventKind::HarnessAnswered,
        EventKind::RemoteOpened,
        EventKind::RemoteClosed,
        EventKind::DevicePaired,
        EventKind::DeviceRevoked,
        EventKind::RemoteSessionStarted,
        EventKind::DocumentWritten,
        EventKind::ProposalOffered,
        EventKind::ProposalDecided,
        EventKind::ProposalWithdrawn,
        EventKind::RunPolicyChanged,
        EventKind::SessionNamed,
        EventKind::SkillAudited,
        EventKind::SignalWaitStarted,
        EventKind::SignalWaitEnded,
        EventKind::SignalLanded,
        EventKind::SkillShelved,
        EventKind::PrefixHeld,
    ];
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
