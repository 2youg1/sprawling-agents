// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, EventKind, EventRecord, NodeId, StopCause};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PlanReach {

    Untouched,

    Stale,

    NodeFreed,

    NodeStopped,
}

pub(super) fn may_move_plan(kind: EventKind) -> PlanReach {
    match kind {
        EventKind::RoadmapFinished | EventKind::RoadmapReleased => PlanReach::NodeFreed,
        EventKind::RoadmapBlocked => PlanReach::NodeStopped,
        EventKind::CityInitialized
        | EventKind::BuildingCreated
        | EventKind::CheckpointCommitted
        | EventKind::RunFrozen
        | EventKind::RoadmapClaimed
        | EventKind::RoadmapSplit

        | EventKind::SpineDocumentWritten => PlanReach::Stale,
        EventKind::BuildingConfigured

        | EventKind::BuildingRemoved
        | EventKind::SessionOpened
        | EventKind::RunStarted
        | EventKind::RunForked
        | EventKind::PromptAssembled

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
        | EventKind::LogTruncated
        | EventKind::SignalEnqueued
        | EventKind::SignalLanded
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
        | EventKind::SignalWaitEnded => PlanReach::Untouched,
    }
}

pub(super) fn building_of(addr: &Address) -> Option<Address> {
    let head = addr.as_str().split('/').next()?;
    Address::parse(head).ok()
}

pub(super) fn node_of(record: &EventRecord) -> Option<NodeId> {
    NodeId::parse(record.data().as_map().get("node")?.as_str()?).ok()
}

pub(super) fn cause_of(record: &EventRecord) -> Option<StopCause> {
    serde_json::from_value(record.data().as_map().get("why")?.clone()).ok()
}
