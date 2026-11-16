// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::EventKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Holding {

    City,

    SignalQueued,

    SignalTaken,

    Pursuit,

    Discarded,

    Restored,

    Claimed,

    Commit,

    RunStarted,

    Asset,

    Ruling,

    Document,

    Prompt,

    PromptShape,

    Nothing,
}

impl Holding {
    pub(super) fn of(kind: EventKind) -> Holding {
        match kind {
            EventKind::CityInitialized => Holding::City,
            EventKind::SignalEnqueued => Holding::SignalQueued,
            EventKind::SignalConsumed => Holding::SignalTaken,
            EventKind::PursuitChanged => Holding::Pursuit,
            EventKind::FileDiscarded => Holding::Discarded,
            EventKind::DiscardRestored => Holding::Restored,
            EventKind::RoadmapClaimed => Holding::Claimed,
            EventKind::CheckpointCommitted | EventKind::PrMerged => Holding::Commit,
            EventKind::RunStarted => Holding::RunStarted,
            EventKind::AssetArchived => Holding::Asset,
            EventKind::ApprovalResolved => Holding::Ruling,
            EventKind::DocumentWritten => Holding::Document,
            EventKind::PromptAssembled => Holding::Prompt,
            EventKind::PromptShapeCompared => Holding::PromptShape,
            EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::BuildingRemoved
            | EventKind::SessionOpened
            | EventKind::RunForked
            | EventKind::ModelCalled
            | EventKind::ModelReturned
            | EventKind::ToolCalled
            | EventKind::ToolResult
            | EventKind::ResultOffloaded
            | EventKind::GateChecked
            | EventKind::GateDenied
            | EventKind::HandoffWritten
            | EventKind::SteerReceived
            | EventKind::CancelReceived
            | EventKind::WatchdogFired
            | EventKind::BudgetLimit
            | EventKind::RunFrozen
            | EventKind::LogTruncated
            | EventKind::DraftHeld
            | EventKind::DraftResolved
            | EventKind::GoalRegistered
            | EventKind::GoalConflict
            | EventKind::ArbitrationVerdict
            | EventKind::RepairStarted
            | EventKind::RepairReused
            | EventKind::WorktreeOpened
            | EventKind::PrOpened
            | EventKind::PrRejected
            | EventKind::RoadmapFinished
            | EventKind::RoadmapReleased
            | EventKind::RoadmapSplit
            | EventKind::RoadmapBlocked
            | EventKind::ApprovalRequested
            | EventKind::PolicyCreated
            | EventKind::PolicyRevoked
            | EventKind::TaintPromoted
            | EventKind::CrossBuildingTransfer
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
            | EventKind::CredentialLent
            | EventKind::SecretCaptured
            | EventKind::SecretEgressBlocked
            | EventKind::AutonomyChanged
            | EventKind::WentBack
            | EventKind::FileRestored
            | EventKind::GovernedDocumentWritten
            | EventKind::SpineDocumentWritten
            | EventKind::RulesChanged
            | EventKind::ToolkitLinkOpened
            | EventKind::EmbeddingCalled
            | EventKind::RerankCalled
            | EventKind::AdviserAsked
            | EventKind::AdviserAnswered
            | EventKind::AdviserFellBack
            | EventKind::CacheRenewed
            | EventKind::PrefixHeld
            | EventKind::HarnessReported
            | EventKind::HarnessAnswered
            | EventKind::RemoteOpened
            | EventKind::RemoteClosed
            | EventKind::DevicePaired
            | EventKind::DeviceRevoked
            | EventKind::RemoteSessionStarted
            | EventKind::ProposalOffered
            | EventKind::ProposalDecided
            | EventKind::ProposalWithdrawn
            | EventKind::RunPolicyChanged
            | EventKind::SessionNamed
            | EventKind::SkillAudited
            | EventKind::SkillShelved
            | EventKind::SignalWaitStarted
            | EventKind::SignalWaitEnded
            | EventKind::SignalLanded => Holding::Nothing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Evidence {

    ToolResult,

    Finished,

    Nothing,
}

impl Evidence {
    pub(super) fn of(kind: EventKind) -> Evidence {
        match kind {
            EventKind::ToolResult => Evidence::ToolResult,
            EventKind::RoadmapFinished => Evidence::Finished,
            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::BuildingRemoved
            | EventKind::SessionOpened
            | EventKind::RunStarted
            | EventKind::RunForked
            | EventKind::PromptAssembled
            | EventKind::PromptShapeCompared
            | EventKind::ModelCalled
            | EventKind::ModelReturned
            | EventKind::ToolCalled
            | EventKind::ResultOffloaded
            | EventKind::GateChecked
            | EventKind::GateDenied
            | EventKind::CheckpointCommitted
            | EventKind::HandoffWritten
            | EventKind::SteerReceived
            | EventKind::CancelReceived
            | EventKind::WatchdogFired
            | EventKind::BudgetLimit
            | EventKind::RunFrozen
            | EventKind::LogTruncated
            | EventKind::SignalEnqueued
            | EventKind::SignalConsumed
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
            | EventKind::AutonomyChanged
            | EventKind::WentBack
            | EventKind::FileRestored
            | EventKind::GovernedDocumentWritten
            | EventKind::SpineDocumentWritten
            | EventKind::RulesChanged
            | EventKind::ToolkitLinkOpened
            | EventKind::EmbeddingCalled
            | EventKind::RerankCalled
            | EventKind::AdviserAsked
            | EventKind::AdviserAnswered
            | EventKind::AdviserFellBack
            | EventKind::CacheRenewed
            | EventKind::PrefixHeld
            | EventKind::HarnessReported
            | EventKind::HarnessAnswered
            | EventKind::RemoteOpened
            | EventKind::RemoteClosed
            | EventKind::DevicePaired
            | EventKind::DeviceRevoked
            | EventKind::RemoteSessionStarted
            | EventKind::DocumentWritten
            | EventKind::ProposalOffered
            | EventKind::ProposalDecided
            | EventKind::ProposalWithdrawn
            | EventKind::RunPolicyChanged
            | EventKind::SessionNamed
            | EventKind::SkillAudited
            | EventKind::SkillShelved
            | EventKind::SignalWaitStarted
            | EventKind::SignalWaitEnded
            | EventKind::SignalLanded => Evidence::Nothing,
        }
    }
}
