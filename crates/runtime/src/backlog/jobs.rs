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
use std::num::{NonZeroU64, NonZeroUsize};

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
    /// The affinity accepted for the job; `Os` when none was requested,
    /// the request was refused, or this platform has no job.
    pub affinity: RunAffinity,
}

/// A group-local affinity supplied by placement (D49), never calculated
/// from configuration inside the runtime.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RunAffinity {
    /// Leave placement to the operating system.
    #[default]
    Os,
    /// Hold the run's job to these processors in the current group.
    Mask(NonZeroUsize),
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
///
/// On Linux the same work is done by a child cgroup per run
/// ([`cgroup`](super::cgroup), D29): its parent is this process's own
/// cgroup, and its reads and writes are plain files.
#[derive(Default)]
pub(super) struct Jobs {
    #[cfg(windows)]
    runs: BTreeMap<RunId, RunJob>,
    #[cfg(not(windows))]
    cgroups: super::cgroup::Cgroups,
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
        table.jobs.enter(&member, self.shares, self.affinity);
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
    pub(super) fn forget(&mut self, owner: RunId) {
        self.cgroups.forget(owner);
    }

    #[cfg(windows)]
    fn enter(&mut self, member: &Member, shares: Shares, affinity: RunAffinity) {
        let Body::Command {
            child,
            claim: Claim::Window(owner) | Claim::Run(owner),
            ..
        } = &member.body
        else {
            return;
        };
        let run = self.runs.entry(*owner).or_default();
        if run.join(child, shares, affinity).is_none() {
            run.unjoined = run.unjoined.saturating_add(1);
        }
    }

    #[cfg(not(windows))]
    fn enter(&mut self, member: &Member, shares: Shares, _affinity: RunAffinity) {
        let Body::Command {
            child,
            claim: Claim::Window(owner) | Claim::Run(owner),
            ..
        } = &member.body
        else {
            return;
        };
        self.cgroups.enter(*owner, child.id(), shares);
    }

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
                reading.affinity = run.affinity;
            } else {
                reading.unfollowed = u32::try_from(reading.pids.len()).unwrap_or(u32::MAX);
            }
        }
    }

    #[cfg(not(windows))]
    fn follow(&self, readings: &mut BTreeMap<RunId, RunProcesses>) {
        for (owner, reading) in readings.iter_mut() {
            reading.unfollowed = u32::try_from(reading.pids.len()).unwrap_or(u32::MAX);
            reading.share = self.cgroups.held(*owner);
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
    affinity: RunAffinity,
}

#[cfg(windows)]
impl RunJob {
    /// Puts `child` into this run's job, creating the job on first use
    /// with the shares asked for. `None` when the job cannot be made or
    /// the process cannot join it.
    fn join(
        &mut self,
        child: &std::process::Child,
        shares: Shares,
        affinity: RunAffinity,
    ) -> Option<()> {
        use std::os::windows::io::AsRawHandle;
        let handle = isize::try_from(child.as_raw_handle().addr()).ok()?;
        let job = match self.job.take() {
            Some(job) => job,
            None => {
                let job = win32job::Job::create().ok()?;
                self.share = given(&job, shares);
                if let RunAffinity::Mask(mask) = affinity {
                    // Read after the memory limit was set: an empty
                    // record here would erase that limit.
                    let applied = job.query_extended_limit_info().and_then(|mut info| {
                        info.limit_affinity(mask.get());
                        job.set_extended_limit_info(&info)
                    });
                    self.affinity = match applied {
                        Ok(()) => affinity,
                        Err(_refused) => RunAffinity::Os,
                    };
                }
                job
            }
        };
        let joined = job.assign_process(handle).ok();
        self.job = Some(job);
        joined
    }
}

/// The shares `job` takes of those `asked`. A job that refuses them still
/// follows the run's processes, and the run is read as unshared; a limit
/// past this process's address space limits nothing, so the job takes
/// the weight alone (D29).
#[cfg(windows)]
fn given(job: &win32job::Job, asked: Shares) -> Shares {
    let memory = match asked {
        Shares::Unset => return Shares::Unset,
        Shares::Cpu => 0,
        Shares::CpuAndMemory { limit } => match usize::try_from(limit.get()) {
            Ok(bytes) => bytes,
            Err(_beyond) => return weigh(job, 0, Shares::Cpu),
        },
    };
    weigh(job, memory, asked)
}

/// Sets the run weight and `memory` (none when zero) on `job`: `held`
/// when the platform takes them, `Unset` when it refuses.
#[cfg(windows)]
fn weigh(job: &win32job::Job, memory: usize, held: Shares) -> Shares {
    match desktop_ffi::cpu::job_share(job.handle(), RUN_CPU_WEIGHT, memory) {
        Ok(()) => held,
        Err(_refused) => Shares::Unset,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::super::Backlog;
    use super::Shares;

    /// The child reads its own affinity after enrolment, so the assertion
    /// observes the running process rather than just a job's limit record.
    #[cfg(windows)]
    #[test]
    fn a_run_child_reads_the_requested_affinity() {
        let available = desktop_ffi::cpu::thread_group().unwrap().mask;
        let mask = std::num::NonZeroUsize::new(
            usize::try_from(1_u64 << available.trailing_zeros()).unwrap(),
        )
        .unwrap();
        let backlog = Backlog::with_window(crate::PollBudget::new(1, 1))
            .with_shares(Shares::CpuAndMemory {
                limit: std::num::NonZeroU64::new(8 << 30).unwrap(),
            })
            .with_affinity(super::RunAffinity::Mask(mask));
        let owner = kernel::RunId::from_bytes([9; 16]);
        let addr = kernel::Address::parse("vault/room1").unwrap();
        for table in [
            backlog.clone(),
            backlog.clone().with_affinity(super::RunAffinity::Os),
        ] {
            let backlog = &table;
            let gate = backlog
                .scratch
                .dir(backlog.mint().unwrap())
                .with_extension("gate");
            let mut child = std::process::Command::new("powershell.exe");
            child.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "while (-not (Test-Path -LiteralPath $env:R05_GATE)) { Start-Sleep -Milliseconds 20 }; [Diagnostics.Process]::GetCurrentProcess().ProcessorAffinity.ToInt64(); exit 23",
    ]);
            child
                .env("R05_GATE", &gate)
                .current_dir(std::env::temp_dir());
            let started = backlog
                .run(owner, &addr, "affinity reader".to_owned(), child)
                .unwrap();
            assert!(matches!(started, crate::Started::Backgrounded { .. }));
            let reading = backlog.processes().unwrap().remove(&owner).unwrap();
            assert_eq!(reading.affinity, super::RunAffinity::Mask(mask));
            assert_eq!(reading.unfollowed, 0);
            assert_eq!(reading.share, backlog.shares());
            std::fs::write(&gate, b"ready").unwrap();
            let mut finished = None;
            for _ in 0..500 {
                if let Some(done) = backlog.harvest(owner).unwrap().into_iter().next() {
                    finished = Some(done);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            std::fs::remove_file(gate).unwrap();
            let finished = finished.expect("the affinity reader finishes");
            assert_eq!(
                finished.exit,
                crate::Exit::Ended { code: 23 },
                "{}",
                finished.stderr
            );
            assert_eq!(finished.stdout.trim().parse::<usize>().unwrap(), mask.get());
        }
        backlog.release(owner);
    }

    /// The same value seam compiles elsewhere without claiming affinity.
    #[cfg(not(windows))]
    #[test]
    fn a_run_affinity_request_elsewhere_keeps_the_command_result() {
        let backlog = Backlog::with_window(crate::PollBudget::new(1, 1)).with_affinity(
            super::RunAffinity::Mask(std::num::NonZeroUsize::new(1).unwrap()),
        );
        let owner = kernel::RunId::from_bytes([9; 16]);
        let addr = kernel::Address::parse("vault/room1").unwrap();
        let mut child = std::process::Command::new("sh");
        child.args(["-c", "sleep 1; echo affinity; exit 23"]);
        child.current_dir(std::env::temp_dir());
        assert!(matches!(
            backlog
                .run(owner, &addr, "reader".to_owned(), child)
                .unwrap(),
            crate::Started::Backgrounded { .. }
        ));
        assert_eq!(
            backlog.processes().unwrap().get(&owner).unwrap().affinity,
            super::RunAffinity::Os
        );
        for _ in 0..250 {
            if let Some(done) = backlog.harvest(owner).unwrap().into_iter().next() {
                assert_eq!(
                    (done.exit, done.stdout.trim()),
                    (crate::Exit::Ended { code: 23 }, "affinity")
                );
                backlog.release(owner);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        backlog.release(owner);
        panic!("the command finishes");
    }

    /// A refused limit stays visible without turning the command into a
    /// failed start or dropping the independently accepted CPU share.
    #[cfg(windows)]
    #[test]
    fn a_refused_run_affinity_keeps_the_command_result_and_share() {
        let available = usize::try_from(desktop_ffi::cpu::thread_group().unwrap().mask).unwrap();
        if available == usize::MAX {
            return; // This group has no invalid affinity bit to ask for.
        }
        let backlog = Backlog::with_window(crate::PollBudget::new(1, 1))
            .with_shares(Shares::Cpu)
            .with_affinity(super::RunAffinity::Mask(std::num::NonZeroUsize::MAX));
        let owner = kernel::RunId::from_bytes([11; 16]);
        let addr = kernel::Address::parse("vault/room1").unwrap();
        let mut child = std::process::Command::new("powershell.exe");
        child.args(["-NoProfile", "-NonInteractive", "-Command", "exit 23"]);
        child.current_dir(std::env::temp_dir());
        assert!(matches!(
            backlog
                .run(owner, &addr, "refused mask".to_owned(), child)
                .unwrap(),
            crate::Started::Backgrounded { .. }
        ));
        let reading = backlog.processes().unwrap().remove(&owner).unwrap();
        assert_eq!(
            (reading.affinity, reading.share, reading.unfollowed),
            (super::RunAffinity::Os, Shares::Cpu, 0)
        );
        for _ in 0..500 {
            if let Some(done) = backlog.harvest(owner).unwrap().into_iter().next() {
                assert_eq!(
                    done.exit,
                    crate::Exit::Ended { code: 23 },
                    "{}",
                    done.stderr
                );
                backlog.release(owner);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        backlog.release(owner);
        panic!("the command finishes despite refusal");
    }
}
