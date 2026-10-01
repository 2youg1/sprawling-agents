// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The window partition: which kinds decide model-request bytes.

use super::EventKind;

/// The two-way partition; the sole criterion is "does the payload decide
/// model-request bytes" (C16). There is no third class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowClass {
    InWindow,
    RecordOnly,
}

impl EventKind {
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
            | EventKind::SignalConsumed
            | EventKind::AdviserAnswered => WindowClass::InWindow,
            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::BuildingRemoved
            | EventKind::RunStarted
            // Starting a session decides what the next run is given, and
            // the bytes it is given are recorded by `prompt_assembled`;
            // nothing here belongs to a window already on the wire.
            | EventKind::SessionOpened
            // The cache shape measures what a request carries; it never
            // decides any of those bytes.
            | EventKind::PromptShapeCompared
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
            // Connecting an application changes which tools a later run
            // is offered, and the tool table is assembled from the
            // configuration rather than from this line, so nothing here
            // decides model-request bytes.
            | EventKind::ToolkitLinkOpened
            // An embedding or a rerank is a call for a derived value,
            // not a turn of the conversation: the window's bytes are
            // recorded by `prompt_assembled`, and the answer here is
            // recorded so a cost can be read off the history.
            | EventKind::EmbeddingCalled
            | EventKind::RerankCalled
            | EventKind::AdviserAsked
            // The fallback records that the city's own policy answered
            // instead; the move it made is already written where the
            // move itself is written.
            | EventKind::AdviserFellBack
            | EventKind::GovernedDocumentWritten
            | EventKind::SpineDocumentWritten
            | EventKind::RulesChanged
            // A renewal resends a prefix already on the wire and changes
            // no byte of any later request; the line records its cost.
            | EventKind::CacheRenewed
            // What a harness said or answered never decides a request
            // this city sends.
            | EventKind::HarnessReported
            | EventKind::HarnessAnswered
            // Who may reach the city from outside its machine decides no
            // byte a model is sent.
            | EventKind::RemoteOpened
            | EventKind::RemoteClosed
            | EventKind::DevicePaired
            | EventKind::DeviceRevoked
            | EventKind::RemoteSessionStarted => WindowClass::RecordOnly,
        }
    }
}
