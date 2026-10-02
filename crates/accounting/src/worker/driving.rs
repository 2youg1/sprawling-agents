// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One drive, and what it leaves behind.

use std::path::PathBuf;

use kernel::RunId;
use kernel::{Address, AxError};
use runtime::bench::ToolBench;
use runtime::run::RunPlan;
use runtime::{SieveSite, package_exec};

use super::{RunWorker, Site};

mod entering;
pub mod flight;
pub(in crate::worker) mod harness;
pub mod lane;
pub(in crate::worker) mod owing;
mod placing;

/// What one drive is handed: the machinery it runs on, and the run it
/// runs as.
///
/// Values that arrive together and are meaningless apart - a bench
/// without the resident that invokes it derives the wrong key, a checkpoint
/// scope without the tree it checkpoints covers the wrong files. They were
/// seven parameters, and `#[expect(clippy::too_many_arguments)]` sat
/// above them saying so.
///
/// **It owns every one of them, and that is what lets it leave this
/// thread** (`crates/sprawling/Spec.lean` §8-46-1). A drive that borrowed the
/// bench, the tree or the resident's name borrowed them from locals of
/// the call that built it, so a lane could never have been handed one.
pub(crate) struct Driving {
    /// The model this run calls, already chosen and already credentialed.
    /// Owned, and handed back inside [`Driven`]: who holds the adapter is
    /// a fact the types state, not a loan the reader has to track.
    pub adapter: super::keeping_warm::Door,
    /// What routes a call the model makes. Spent by the drive: nothing
    /// after it asks the bench anything, so it is dropped where it is
    /// used rather than carried home.
    pub bench: ToolBench,
    /// Where a steer from a resident lands while the drive is going.
    /// A handle of its own rather than a loan: the drive may leave the
    /// thread that opened the desk (`crates/sprawling/Spec.lean` §8-44).
    pub signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    /// The tree the run writes in: its own worktree under review, the
    /// city itself otherwise.
    pub write_root: PathBuf,
    /// What a checkpoint covers, from [`Site::checkpoint_scope`]:
    /// every prefix of the run's write domain, not just its room.
    pub checkpoint_scope: Vec<String>,
    pub run_id: RunId,
    /// What every checkpoint this drive raises is signed with.
    pub of: storage::Provenance,
    /// Where a command's output is pinned before it is cut, and what
    /// decides the cut (`crates/sprawling/Spec.lean` §8-43).
    pub sieving: Sieving,
    /// This run's place in the backlog, when it is a run somebody handed
    /// down: the member a halt on its scope marks (`crates/runtime/Spec.lean` §8-28-2).
    pub member: Option<runtime::BacklogId>,
    /// What is to be run, and what says how to pick it up again. Fields
    /// rather than parameters beside this value: a lane is entered with
    /// one thing.
    pub plan: RunPlan,
    pub handoff: runtime::handoff::Handoff,
    /// The rest of the bench, which the drive carries home for the
    /// landing to read: who the run handed work to, and whether it asked
    /// to be replaced.
    pub(in crate::worker) workbench: super::Workbench,
}

/// What the sieve needs from the city for one run: a store to pin the
/// original in, the room the model reads the rest from, the
/// filter table frozen with the run, and what this run already saw;
/// and what decides a command result's clock line: the run's gate and
/// the driver's latest reading (`crates/sprawling/Spec.lean` §8-125).
pub(crate) struct Sieving {
    /// The store the lanes share, taken for the length of one package.
    pub cas: std::sync::Arc<std::sync::Mutex<storage::Cas>>,
    pub city_root: PathBuf,
    pub room: Address,
    /// The run and room an original is pinned for.
    pub origin: storage::BlockOrigin,
    pub table: runtime::FilterTable,
    pub history: runtime::SieveHistory,
    /// Whether a result carries the clock line, built from the run's
    /// frozen `[clock]`.
    pub stamps: runtime::StampGate,
    /// What the driver read last. The drive's clock hook keeps every
    /// reading here, so a stamp names a moment the ledger records and
    /// the run still has one sampling point.
    pub clock: runtime::ClockReading,
}

impl Sieving {
    /// What the model reads of one command's output, decided by the
    /// pipeline under the command's own key with the original pinned
    /// first, and its clock line where the run's gate asks for one.
    /// Before the driver has read its clock there is nothing to stamp.
    fn package(
        &mut self,
        call: &kernel::ToolCall,
        outcome: kernel::ToolOutcome,
        temporal: kernel::Temporal,
    ) -> Result<kernel::ToolOutcome, AxError> {
        let stamp = match self.clock.latest() {
            Some(at) => self.stamps.observe(at, temporal)?,
            None => None,
        };
        let mut cas = super::held(&self.cas, "take the lanes' store")?;
        package_exec(
            call,
            outcome,
            SieveSite {
                offload: runtime::offload::OffloadSite {
                    cas: &mut cas,
                    city_root: &self.city_root,
                    room: &self.room,
                    origin: self.origin.clone(),
                },
                table: &self.table,
                history: &mut self.history,
            },
            stamp,
        )
    }
}

impl Sieving {
    /// What the model reads of one connector answer: the pipeline's
    /// window over it, with the original stored where it had to be cut.
    pub(super) fn package_connector(
        &mut self,
        outcome: kernel::ToolOutcome,
    ) -> Result<kernel::ToolOutcome, AxError> {
        let mut cas = super::held(&self.cas, "take the lanes' store")?;
        runtime::package_connector(
            outcome,
            runtime::offload::OffloadSite {
                cas: &mut cas,
                city_root: &self.city_root,
                room: &self.room,
                origin: self.origin.clone(),
            },
        )
    }
}

/// What one drive left behind, beside the run it froze.
///
/// Every field is written by a hook while the driver owns the ledger and
/// read after it gives it back, so none of them may be acted on until the
/// drive has returned.
pub struct Driven {
    pub outcome: Result<runtime::Run<runtime::run::Frozen>, AxError>,
    /// The adapter, home from the drive. The caller takes it apart back
    /// into the site it came from: a `Site` gains and loses no field.
    pub adapter: super::keeping_warm::Door,
    /// The commits each wave checkpointed against; the first is what the sweep
    /// restores a discarded file from.
    pub checkpointed: Vec<String>,
    /// The run's own commands, as (passed, failed).
    pub ran: (u32, u32),
    /// What the drive raised for a person. Empty today: every door
    /// answers Allow or Deny, and the only question left is the
    /// sweep's, which is raised after the drive hands the ledger back.
    pub raised: Vec<kernel::ApprovalItem>,
    /// The bench the drive was handed, home for the landing.
    pub(in crate::worker) workbench: super::Workbench,
}

impl Sieving {
    /// What the sieve needs from this city for one run.
    ///
    /// The store is the handle the lanes share rather than a loan of
    /// the worker's: a drive that borrowed the worker's could not leave
    /// the thread the worker lives on, and a handle opened here would
    /// sweep the half-written objects of every other lane's put. The
    /// rest directory sits inside the room, which is the one place a
    /// model-chosen path is allowed to read from; the offload makes it
    /// when it first writes a rest file.
    pub(in crate::worker) fn for_run(
        store: &std::sync::Arc<std::sync::Mutex<storage::Cas>>,
        site: &Site,
        addr: &Address,
    ) -> Sieving {
        Sieving {
            cas: std::sync::Arc::clone(store),
            city_root: site.write_root.clone(),
            room: addr.clone(),
            origin: storage::BlockOrigin {
                run: site.run_id,
                building: addr.clone(),
            },
            table: site.filters.clone(),
            history: runtime::SieveHistory::default(),
            stamps: runtime::StampGate::new(
                site.config.clock_stamp,
                site.config.clock_zones.clone(),
            ),
            clock: site.clock.clone(),
        }
    }
}

impl RunWorker {
    /// The four handles a drive takes from this worker.
    ///
    /// Cloned rather than lent, so N drives can hold them at once and
    /// none of them holds the worker. The ledger is deliberately absent:
    /// which ledger a drive writes through is what separates the
    /// accounting thread from a lane (`crates/sprawling/Spec.lean` §8-46-1).
    pub(in crate::worker) fn drive_context(&self) -> lane::DriveContext {
        lane::DriveContext {
            watching: self
                .serving
                .as_ref()
                .map(|at| std::sync::Arc::clone(&at.deltas)),
            person: self
                .serving
                .as_ref()
                .map(|at| std::sync::Arc::clone(&at.interrupts)),
            checkpoint_gate: std::sync::Arc::clone(&self.flight.checkpoint_gate),
            backlog: self.flight.backlog.clone(),
            clock: std::sync::Arc::clone(&self.clock),
        }
    }
}

#[cfg(test)]
mod tests;
