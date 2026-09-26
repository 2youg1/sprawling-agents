// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each room's current session branched from, if it branched.
//!
//! **A branch belongs to the session and is spent by the run that
//! begins it.** A session is a stretch of a room, and the person who
//! started it may have said "continue from there": that is a fact about
//! the session, so it is recorded once, when the session opens, and not
//! sent again with every dispatch. The first run started afterwards is
//! the one that inherits the conversation - a branch is a beginning -
//! and the line that records that run's lineage is what marks the
//! inheritance spent, which is why this fold reads two kinds and holds
//! one map:
//!
//! - `session_opened` carries the origin and replaces whatever the room
//!   had pending, including with nothing: a session that began without
//!   one inherits nothing, whatever the session before it did.
//! - `run_forked` is that lineage line. A run that continues another
//!   from a line of it has inherited as much as there is to inherit, so
//!   the room has nothing pending afterwards.
//!
//! **The map is not a second authority for the lineage.** What run
//! continues what is in the ledger, and stays there; this holds only
//! the question a dispatch asks before it has a run to name - "is this
//! session still owed a conversation?" - and a rebuild answers it from
//! the same two kinds in the same order.

use std::collections::BTreeMap;

use kernel::{Address, AxError, EventKind, Origin};

/// What each room's current session branched from, and whether the run
/// that begins it has been started yet.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(in crate::assembly) struct SessionOrigins {
    pending: BTreeMap<Address, Origin>,
}

impl SessionOrigins {
    /// One record, folded for what it says about a session's origin.
    ///
    /// **Written for one record at a time, not one `EventRecord`.** The
    /// rebuild has verified records and the worker has the three fields a
    /// record is made of, and asking both to build the other's shape
    /// would be two readings of one line.
    ///
    /// # Errors
    /// Refuses a `session_opened` payload whose `from` cannot be read as
    /// an origin: a session whose beginning does not say what it branched
    /// from would otherwise be read as one that branched from nothing,
    /// which is a different session.
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        addr: Option<&Address>,
        data: &kernel::Payload,
    ) -> Result<(), AxError> {
        match kind {
            EventKind::SessionOpened => {
                let Some(addr) = addr else {
                    return Ok(());
                };
                let opened = crate::views::session_opened(data)?;
                match opened {
                    Some(origin) => {
                        self.pending.insert(addr.clone(), origin);
                    }
                    None => {
                        self.pending.remove(addr);
                    }
                }
                Ok(())
            }
            EventKind::RunForked => {
                if let Some(addr) = addr {
                    self.spent(addr);
                }
                Ok(())
            }
            // Everything else is not a session beginning: listed
            // rather than defaulted, so a kind added to the vocabulary
            // answers here whether it opens one.
            EventKind::CityInitialized
            | EventKind::BuildingCreated
            | EventKind::BuildingConfigured
            | EventKind::RunStarted
            | EventKind::PromptAssembled
            | EventKind::PromptShapeCompared
            | EventKind::ModelCalled
            | EventKind::ModelReturned
            | EventKind::ToolCalled
            | EventKind::ToolResult
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
            | EventKind::GovernedDocumentWritten
            | EventKind::SpineDocumentWritten
            | EventKind::ToolkitLinkOpened
            | EventKind::EmbeddingCalled
            | EventKind::RerankCalled
            | EventKind::AdviserAsked
            | EventKind::AdviserAnswered
            | EventKind::AdviserFellBack => Ok(()),
        }
    }

    /// What this room's session is still owed, if anything.
    pub(in crate::assembly) fn get(&self, addr: &Address) -> Option<Origin> {
        self.pending.get(addr).copied()
    }

    /// Marks the inheritance spent: the run that begins the session has
    /// been written, so no later run of it inherits anything.
    ///
    /// Called by the fold on the lineage line and directly by the
    /// dispatch that writes one, because a worker is not shown its own
    /// appends by the fold that reads the history back.
    pub(in crate::assembly) fn spent(&mut self, addr: &Address) {
        self.pending.remove(addr);
    }
}
