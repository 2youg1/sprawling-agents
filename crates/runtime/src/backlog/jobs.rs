// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which processes belong to which run (runtime-SPEC 8-13-3).
//!
//! A dispatched `cargo test` spends its memory in the `rustc` and test
//! processes it starts, not in `cargo`. On Windows every run therefore
//! gets one anonymous Job Object, and each command the backlog starts
//! for that run joins it the moment the command enters the table; the
//! processes a member of a job starts join the same job by themselves,
//! so the job's process list is the run's whole process tree. The job
//! is not kill-on-close: dropping it at `release` stops nothing, because
//! stopping belongs to `release` and `halt` alone.

use std::collections::{BTreeMap, BTreeSet};

use kernel::{AxError, RunId};

use super::{Backlog, BacklogId, Body, Claim, Member};

/// The processes one run has in the backlog right now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunProcesses {
    /// Each command's own pid, and every process its run's job holds.
    /// A command that ended but was not yet harvested is still listed;
    /// a reader finds no counters behind its pid.
    pub pids: BTreeSet<u32>,
    /// How many of the run's commands were read as themselves only,
    /// without the processes they started: on Windows the ones that
    /// could not join the job (all of them when the job's list could
    /// not be read), and every one of them elsewhere, where there is
    /// no Job Object.
    pub unfollowed: u32,
}

/// Each run's job, and how many of its commands did not get into it.
#[derive(Default)]
pub(super) struct Jobs {
    #[cfg(windows)]
    runs: BTreeMap<RunId, RunJob>,
}

impl Backlog {
    /// The processes of every run that has a command in the table.
    /// A command released to nobody belongs to no run.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the table cannot be reached.
    pub fn processes(&self) -> Result<BTreeMap<RunId, RunProcesses>, AxError> {
        let table = self.hold()?;
        let mut runs: BTreeMap<RunId, RunProcesses> = BTreeMap::new();
        for member in table.members.values() {
            if let Body::Command {
                child,
                claim: Claim::Window(owner) | Claim::Run(owner),
                ..
            } = &member.body
            {
                runs.entry(*owner).or_default().pids.insert(child.id());
            }
        }
        table.jobs.follow(&mut runs);
        Ok(runs)
    }

    /// Enters a member into the table, and a command into its run's job.
    pub(super) fn enrol(&self, id: BacklogId, member: Member) -> Result<(), AxError> {
        let mut table = self.hold()?;
        table.jobs.enter(&member);
        table.members.insert(id, member);
        Ok(())
    }
}

impl Jobs {
    /// Lets the run's job go; its processes keep running.
    #[cfg(windows)]
    pub(super) fn forget(&mut self, owner: RunId) {
        self.runs.remove(&owner);
    }

    #[cfg(not(windows))]
    pub(super) fn forget(&mut self, _owner: RunId) {}

    #[cfg(windows)]
    fn enter(&mut self, member: &Member) {
        let Body::Command {
            child,
            claim: Claim::Window(owner) | Claim::Run(owner),
            ..
        } = &member.body
        else {
            return;
        };
        let run = self.runs.entry(*owner).or_default();
        if run.join(child).is_none() {
            run.unjoined = run.unjoined.saturating_add(1);
        }
    }

    #[cfg(not(windows))]
    fn enter(&mut self, _member: &Member) {}

    #[cfg(windows)]
    fn follow(&self, readings: &mut BTreeMap<RunId, RunProcesses>) {
        for reading in readings.values_mut() {
            reading.unfollowed = u32::try_from(reading.pids.len()).unwrap_or(u32::MAX);
        }
    }

    #[cfg(not(windows))]
    fn follow(&self, readings: &mut BTreeMap<RunId, RunProcesses>) {
        for reading in readings.values_mut() {
            reading.unfollowed = u32::try_from(reading.pids.len()).unwrap_or(u32::MAX);
        }
    }
}

/// One run's job, created when its first command joins.
#[cfg(windows)]
#[derive(Default)]
struct RunJob {
    job: Option<win32job::Job>,
    unjoined: u32,
}

#[cfg(windows)]
impl RunJob {
    /// Puts `child` into this run's job, creating the job on first use.
    /// `None` when the job cannot be made or the process cannot join it.
    fn join(&mut self, child: &std::process::Child) -> Option<()> {
        use std::os::windows::io::AsRawHandle;
        let handle = isize::try_from(child.as_raw_handle().addr()).ok()?;
        let job = match self.job.take() {
            Some(job) => job,
            None => win32job::Job::create().ok()?,
        };
        let joined = job.assign_process(handle).ok();
        self.job = Some(job);
        joined
    }
}
