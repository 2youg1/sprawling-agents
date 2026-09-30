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
//! A session opened with `--carry` is owed one more thing, the address
//! of the previous run's transcript, and it is owed it the same way: the
//! room's last `run_started` names the run, `session_opened` with
//! `carried` marks it owed, and the next `run_started` there spends it.
//!
//! **The map is not a second authority for the lineage.** What run
//! continues what is in the ledger, and stays there; this holds only
//! the question a dispatch asks before it has a run to name - "is this
//! session still owed a conversation?" - and a rebuild answers it from
//! the same two kinds in the same order.

use std::collections::BTreeMap;

use kernel::{Address, AxError, EventKind, Origin, RunId};

/// What each room's current session branched from, and whether the run
/// that begins it has been started yet.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(in crate::assembly) struct SessionOrigins {
    pending: BTreeMap<Address, Origin>,
    /// The run each room started last.
    last_run: BTreeMap<Address, RunId>,
    /// The run whose transcript a carried session's first run is told about.
    carried: BTreeMap<Address, RunId>,
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
    /// which is a different session. A line written before `from`
    /// existed reads as a session without a branch, which is what it was.
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        run: RunId,
        addr: Option<&Address>,
        data: &kernel::Payload,
    ) -> Result<(), AxError> {
        match kind {
            EventKind::SessionOpened => {
                let Some(addr) = addr else {
                    return Ok(());
                };
                let opened = data.read::<kernel::event::record::SessionOpened>()?;
                match opened.from {
                    Some(origin) => {
                        self.pending.insert(addr.clone(), origin);
                    }
                    None => {
                        self.pending.remove(addr);
                    }
                }
                match self.last_run.get(addr).filter(|_| opened.carried) {
                    Some(last) => {
                        self.carried.insert(addr.clone(), *last);
                    }
                    None => {
                        self.carried.remove(addr);
                    }
                }
                Ok(())
            }
            EventKind::RunStarted => {
                if let Some(addr) = addr {
                    self.started(addr, run);
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
            | EventKind::BuildingRemoved
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
            | EventKind::HarnessAnswered => Ok(()),
        }
    }

    /// What this room's session is still owed, if anything.
    pub(in crate::assembly) fn get(&self, addr: &Address) -> Option<Origin> {
        self.pending.get(addr).copied()
    }

    /// The run whose transcript this room's carried session has not yet
    /// named to a run of its own, if any.
    pub(in crate::assembly) fn carried_from(&self, addr: &Address) -> Option<RunId> {
        self.carried.get(addr).copied()
    }

    /// Records `run` as the room's last run, which spends what a carried
    /// session was owed.
    ///
    /// Called by the fold on `run_started` and directly by the freeze
    /// that begins a run, because the runtime writes that line and the
    /// worker is not shown it.
    pub(in crate::assembly) fn started(&mut self, addr: &Address, run: RunId) {
        self.carried.remove(addr);
        self.last_run.insert(addr.clone(), run);
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
