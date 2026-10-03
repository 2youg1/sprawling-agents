// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each event kind is to a question the views ask
//! (`crates/accounting/spec/Views.lean` D49).
//!
//! One table per question, each naming every kind, so a kind the kernel
//! adds is a compile error here rather than a record that silently
//! changes nothing. The callers match on the class exhaustively.

use kernel::EventKind;

/// What a record of each kind changes in what the views hold
/// (`Views::apply`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Holding {
    /// The city's own address.
    City,
    /// A signal joins its room's queue.
    SignalQueued,
    /// A signal leaves every queue.
    SignalTaken,
    /// A room's pursuit is set or cleared.
    Pursuit,
    /// Files are set aside.
    Discarded,
    /// Files set aside come back.
    Restored,
    /// A run claims a plan node.
    Claimed,
    /// Work lands in a commit.
    Commit,
    /// A run starts: its predecessor and its skill pins.
    RunStarted,
    /// An asset enters the registry.
    Asset,
    /// An approval is answered.
    Ruling,
    /// A document is saved.
    Document,
    /// A run's first prompt.
    Prompt,
    /// Nothing the views hold beyond the folds every record passes.
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
            EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::BuildingRemoved
            | EventKind::SessionOpened
            | EventKind::RunForked
            | EventKind::PromptShapeCompared
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
            | EventKind::SignalWaitStarted
            | EventKind::SignalWaitEnded
            | EventKind::SignalLanded => Holding::Nothing,
        }
    }
}

/// Which records carry a locator a reader can check a run by
/// (`evidence_in`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Evidence {
    /// A tool result, which carries one when it stored a picture.
    ToolResult,
    /// A plan node closed with its completion.
    Finished,
    /// No locator.
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
            | EventKind::SignalWaitStarted
            | EventKind::SignalWaitEnded
            | EventKind::SignalLanded => Evidence::Nothing,
        }
    }
}
