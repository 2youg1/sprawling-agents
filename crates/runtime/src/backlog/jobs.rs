// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which processes belong to which run (`crates/runtime/spec/Tools/Exec.lean` §8-13-3).
//!
//! A dispatched `cargo test` spends its memory in the `rustc` and test
//! processes it starts, not in `cargo`. On Windows every run therefore
//! gets one anonymous Job Object, and each command the backlog starts
//! for that run joins it the moment the command enters the table; the
//! processes a member of a job starts join the same job by themselves,
//! so the job's process list is the run's whole process tree. The job
//! is not kill-on-close: dropping it at `release` stops nothing, because
//! stopping belongs to `release` and `halt` alone.
//!
//! Each job is made with the shares the backlog was given (D29 in the
//! same part): a CPU weight, the same for every run, so a run whose build
//! starts sixteen compilers takes one share of the processors and not
//! sixteen, and with it, when asked, a limit on the memory the run's
//! processes commit together.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

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
    /// The shares the run's processes hold right now: the ones the
    /// backlog asked for when the platform gave them, otherwise `Unset`.
    pub share: Shares,
}

/// How a run's processes share this machine with the other runs'
/// processes (D29). The backlog is given the shares to ask for by the
/// assembly, which takes them from the person's `[core] placement`
/// (`crates/sprawling/spec/Serving/Placement.lean` D47); this crate reads
/// no setting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Shares {
    /// Nothing set: the processes compete thread by thread. Also what a
    /// run reads as when the platform has no shares or refused them.
    #[default]
    Unset,
    /// Every run's processes hold the same CPU weight.
    Cpu,
    /// The same CPU weight, and all of one run's processes together
    /// commit at most `limit` bytes.
    CpuAndMemory {
        /// The bytes the run's processes may commit together.
        limit: NonZeroU64,
    },
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
        for (owner, run) in &self.runs {
            let reading = readings.entry(*owner).or_default();
            let listed = run.job.as_ref().map(win32job::Job::query_process_id_list);
            if let Some(Ok(pids)) = listed {
                // A Windows pid is a DWORD; the list only widens it.
                reading
                    .pids
                    .extend(pids.into_iter().filter_map(|pid| u32::try_from(pid).ok()));
                reading.unfollowed = reading.unfollowed.saturating_add(run.unjoined);
                reading.share = run.share;
            } else {
                reading.unfollowed = u32::try_from(reading.pids.len()).unwrap_or(u32::MAX);
            }
        }
    }

    #[cfg(not(windows))]
    fn follow(&self, readings: &mut BTreeMap<RunId, RunProcesses>) {
        for reading in readings.values_mut() {
            reading.unfollowed = u32::try_from(reading.pids.len()).unwrap_or(u32::MAX);
        }
    }
}

/// The weight every run's job holds, from 1 to 9: the same for every
/// run, so the runs share the processors evenly among themselves.
#[cfg(windows)]
const RUN_CPU_WEIGHT: u32 = 5;

/// One run's job, created when its first command joins.
#[cfg(windows)]
#[derive(Default)]
struct RunJob {
    job: Option<win32job::Job>,
    unjoined: u32,
    share: Shares,
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
            None => {
                let job = win32job::Job::create().ok()?;
                // A job that refuses the weight still follows the run's
                // processes; the run is read as unshared (D29).
                self.share = match desktop_ffi::cpu::job_share(job.handle(), RUN_CPU_WEIGHT, 0) {
                    Ok(()) => Shares::Cpu,
                    Err(_refused) => Shares::Unset,
                };
                job
            }
        };
        let joined = job.assign_process(handle).ok();
        self.job = Some(job);
        joined
    }
}
