// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Event kinds: the closed set and its window classes.

use serde::{Deserialize, Serialize};

/// The closed event vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum EventKind {
    // Genesis and space (3).
    CityInitialized,
    BuildingCreated,
    /// A person changed what one building's runs may reach.
    ///
    /// The payload says which faces were written, never what they now
    /// hold: `CONFIG.toml` is the authority for what a run is governed
    /// by, and a copy of it here would be a second one. What this adds
    /// is the fact the file cannot carry - that the change happened,
    /// when, and to which building.
    BuildingConfigured,
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
    // Governance and facilities (18).
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
    /// One of the three documents that govern a city was written by a
    /// person. Recorded because what governs a city decides what every
    /// later run is given, and "who changed this, and when" has to be
    /// answerable from the one history rather than from a file's
    /// modification time.
    GovernedDocumentWritten,
    /// A person asked to connect an outside application, and the broker
    /// that holds its OAuth opened a consent session.
    ///
    /// The request is history and belongs here; **where that application
    /// stands right now is not, and is deliberately absent from the
    /// payload** (channels-SPEC.md section 8-31): a recorded standing
    /// would still read "connected" an hour after the person revoked
    /// it. The consent page's own url is absent for a second reason -
    /// it is a capability, and anybody replaying this log would be
    /// holding one.
    ToolkitLinkOpened,

    // The two faces of an endpoint that are not the conversation (2).
    /// One call to an attached endpoint's embeddings face, and what came
    /// back for it. The vectors are absent for the reason `ModelCalled`
    /// carries no request body: a replay recomputes them from the inputs
    /// the call recorded.
    EmbeddingCalled,
    /// One call to an attached endpoint's rerank face, and how many
    /// passages and ranks it was about.
    RerankCalled,

    // The adviser port, consulted about the window (3).
    /// A judgment was asked of the adviser port about the window.
    ///
    /// Asking decides nothing on its own: whether the bytes the model
    /// reads include the item in question is decided by the answer, or
    /// by the fallback when none came.
    AdviserAsked,
    /// The adviser answered. This is the line a replay reads instead of
    /// asking again, which is why the answer decides model-request bytes
    /// and the question that produced it does not.
    AdviserAnswered,
    /// No adviser answer was used, and why. The deterministic strategy
    /// answered instead, and this is the only line that says so: without
    /// it a window an adviser shaped and a window the city's own policy
    /// shaped would fold to the same history.
    AdviserFellBack,
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
    pub const ALL: [EventKind; 72] = [
        EventKind::CityInitialized,
        EventKind::BuildingCreated,
        EventKind::BuildingConfigured,
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
        EventKind::GovernedDocumentWritten,
        EventKind::ToolkitLinkOpened,
        EventKind::EmbeddingCalled,
        EventKind::RerankCalled,
        EventKind::AdviserAsked,
        EventKind::AdviserAnswered,
        EventKind::AdviserFellBack,
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
            | EventKind::SignalConsumed
            | EventKind::AdviserAnswered => WindowClass::InWindow,
            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::BuildingConfigured
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
            | EventKind::AutonomyChanged
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
            | EventKind::GovernedDocumentWritten => WindowClass::RecordOnly,
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
mod tests;
