// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How far one record reaches into the plans: whether its kind can move
//! a plan, which building it names, and the node and stop cause it
//! carries (`crates/sprawling/Spec.lean` §8-76).

use kernel::{Address, EventKind, EventRecord, NodeId, StopCause};

/// How far one record reaches into what this view holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PlanReach {
    /// The kind cannot move a plan, so every parsed copy still stands.
    Untouched,
    /// The plan may have moved and the parsed copy is out of date.
    Stale,
    /// Out of date, and the node the record names is no longer stopped.
    NodeFreed,
    /// Out of date, and the node the record names has stopped, for the
    /// reason the record carries.
    NodeStopped,
}

/// Which records can move a plan. Exhaustive on purpose: a kind added
/// to the vocabulary without an answer here is a compile error rather
/// than a stale table nobody notices (`crates/sprawling/Spec.lean` §8-76).
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
        // A person's write to `Roadmap.md` moves the plan like any
        // other, so the table is re-read rather than trusted.
        | EventKind::SpineDocumentWritten => PlanReach::Stale,
        EventKind::BuildingConfigured
        // A removed building is no longer listed, so no view asks for
        // its plan; a building raised later under the same name is a
        // `building_created`, which re-reads.
        | EventKind::BuildingRemoved
        | EventKind::SessionOpened
        | EventKind::RunStarted
        | EventKind::RunForked
        | EventKind::PromptAssembled
        // The cache shape measures one request; it names no plan row.
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
        // A call to an embeddings or rerank face, and every line an
        // adviser consultation writes, are about the window one run is
        // given: none of them names a plan row.
        | EventKind::EmbeddingCalled
        | EventKind::RerankCalled
        | EventKind::AdviserAsked
        | EventKind::AdviserAnswered
        | EventKind::AdviserFellBack
        | EventKind::CacheRenewed
        | EventKind::HarnessReported
        | EventKind::HarnessAnswered
        // Who may reach the city from outside moves no plan.
        | EventKind::RemoteOpened
        | EventKind::RemoteClosed
        | EventKind::DevicePaired
        | EventKind::DeviceRevoked
        | EventKind::RemoteSessionStarted
        // A saved document and a proposal about one move no plan row.
        | EventKind::DocumentWritten
        | EventKind::ProposalOffered
        | EventKind::ProposalDecided
        | EventKind::ProposalWithdrawn
        // A run's policy, a session's name and a skill's audit move no
        // plan row.
        | EventKind::RunPolicyChanged
        | EventKind::SessionNamed
        | EventKind::SkillAudited
        // A run waiting for a reply moves no plan row.
        | EventKind::SignalWaitStarted
        | EventKind::SignalWaitEnded => PlanReach::Untouched,
    }
}

/// The building an address belongs to: its first segment.
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
