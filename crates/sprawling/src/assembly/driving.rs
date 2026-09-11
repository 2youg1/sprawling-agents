// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One drive, and what it leaves behind.

use std::path::PathBuf;

use kernel::RunId;
use kernel::{Address, AxCode, AxError};
use runtime::bench::ToolBench;
use runtime::run::RunPlan;
use runtime::{SieveSite, package_exec};

use super::{RunWorker, Site};

pub(crate) mod flight;
pub(crate) mod lane;

/// What one drive is handed: the machinery it runs on, and the run it
/// runs as.
///
/// Values that arrive together and are meaningless apart - a bench
/// without the resident that invokes it derives the wrong key, a fence
/// scope without the tree it fences covers the wrong files. They were
/// seven parameters, and `#[expect(clippy::too_many_arguments)]` sat
/// above them saying so.
///
/// **It owns every one of them, and that is what lets it leave this
/// thread** (sprawling-SPEC.md 8-46-1). A drive that borrowed the
/// bench, the tree or the resident's name borrowed them from locals of
/// the call that built it, so a lane could never have been handed one.
pub(crate) struct Driving {
    /// The model this run calls, already chosen and already credentialed.
    /// Owned, and handed back inside [`Driven`]: who holds the adapter is
    /// a fact the types state, not a loan the reader has to track.
    pub(crate) adapter: Box<dyn kernel::Model + Send>,
    /// What routes a call the model makes. Spent by the drive: nothing
    /// after it asks the bench anything, so it is dropped where it is
    /// used rather than carried home.
    pub(crate) bench: ToolBench,
    /// Where a steer from a resident lands while the drive is going.
    /// A handle of its own rather than a loan: the drive may leave the
    /// thread that opened the desk (sprawling-SPEC 8-44).
    pub(crate) signals: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    /// The tree the run writes in: its own worktree under review, the
    /// city itself otherwise.
    pub(crate) write_root: PathBuf,
    /// What a checkpoint fence covers, from [`Site::fence_scope`]:
    /// every prefix of the run's write domain, not just its room.
    pub(crate) fence_scope: Vec<String>,
    /// The resident this run works as, as three hooks will name it.
    pub(crate) who: String,
    pub(crate) run_id: RunId,
    /// What every fence this drive raises is signed with.
    pub(crate) of: memory::Provenance,
    /// Where a command's output is pinned before it is cut, and what
    /// decides the cut (sprawling-SPEC 8-43).
    pub(crate) sieving: Sieving,
    /// This run's place in the backlog, when it is a run somebody handed
    /// down: the member a halt on its scope marks (runtime-SPEC 8-28-2).
    pub(crate) member: Option<runtime::BacklogId>,
    /// What is to be run, and what says how to pick it up again. Fields
    /// rather than parameters beside this value: a lane is entered with
    /// one thing.
    pub(crate) plan: RunPlan,
    pub(crate) handoff: runtime::handoff::Handoff,
}

/// What the sieve needs from the city for one run: a store to pin the
/// original in, a directory the model can read the rest from, the
/// filter table frozen with the run, and what this run already saw.
pub(crate) struct Sieving {
    pub(crate) cas: memory::Cas,
    pub(crate) environment: PathBuf,
    pub(crate) table: runtime::FilterTable,
    pub(crate) history: runtime::SieveHistory,
}

impl Sieving {
    /// What the model reads of one command's output, decided by the
    /// pipeline under the command's own key with the original pinned
    /// first. The stamp is `None` because the turn already stamps a
    /// result where the frozen configuration asks for one.
    fn package(
        &mut self,
        call: &kernel::ToolCall,
        outcome: kernel::ToolOutcome,
    ) -> Result<kernel::ToolOutcome, AxError> {
        package_exec(
            call,
            outcome,
            SieveSite {
                offload: runtime::offload::OffloadSite {
                    cas: &mut self.cas,
                    environment: &self.environment,
                },
                table: &self.table,
                history: &mut self.history,
            },
            None,
        )
    }
}

/// What one drive left behind, beside the run it froze.
///
/// Every field is written by a hook while the driver owns the ledger and
/// read after it gives it back, so none of them may be acted on until the
/// drive has returned.
pub(crate) struct Driven {
    pub(crate) outcome: Result<runtime::Run<runtime::run::Frozen>, AxError>,
    /// The adapter, home from the drive. The caller takes it apart back
    /// into the site it came from: a `Site` gains and loses no field.
    pub(crate) adapter: Box<dyn kernel::Model + Send>,
    /// The commits each wave fenced against; the first is what the sweep
    /// restores a discarded file from.
    pub(crate) fenced: Vec<String>,
    /// The run's own commands, as (passed, failed).
    pub(crate) ran: (u32, u32),
    /// What a gate escalated while the ledger was not the worker's.
    pub(crate) raised: Vec<kernel::ApprovalItem>,
}

impl RunWorker {
    /// What the sieve needs from this city for one run.
    ///
    /// The store is a second handle on the same CAS rather than a loan
    /// of the worker's: the store is content-addressed and written
    /// through a temporary file, so two handles are one library, and a
    /// drive that borrowed the worker's could not leave the thread the
    /// worker lives on. The rest directory sits inside the room, which
    /// is the one place a model-chosen path is allowed to read from.
    ///
    /// # Errors
    /// Propagates a store that will not open and a rest directory that
    /// cannot be made.
    pub(in crate::assembly) fn sieving_for(
        &self,
        site: &Site,
        addr: &Address,
    ) -> Result<Sieving, AxError> {
        let cas = memory::Cas::open(&self.city_root.join(".sprawling").join("cas"))
            .map_err(memory::MemoryError::into_ax)?;
        let environment = site.write_root.join(addr.as_str()).join(".rest");
        std::fs::create_dir_all(&environment).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "make the rest directory",
                format!("{}: {err}", environment.display()),
            )
            .with_recovery("make the room writable")
        })?;
        Ok(Sieving {
            cas,
            environment,
            table: site.filters.clone(),
            history: runtime::SieveHistory::default(),
        })
    }

    /// The four handles a drive takes from this worker.
    ///
    /// Cloned rather than lent, so N drives can hold them at once and
    /// none of them holds the worker. The ledger is deliberately absent:
    /// which ledger a drive writes through is what separates the
    /// accounting thread from a lane (sprawling-SPEC.md 8-46-1).
    pub(in crate::assembly) fn drive_context(&self) -> lane::DriveContext {
        lane::DriveContext {
            watching: self.watching.clone(),
            person: self.interrupts.clone(),
            backlog: self.backlog.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
