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

use super::ceiling::{Ceiling, Mark, Unapplied};
#[cfg(windows)]
use super::run_job::{RUN_CPU_WEIGHT, RunJob};
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

impl Shares {
    /// The memory ceiling these shares ask for, if any.
    pub(super) fn memory(self) -> Option<NonZeroU64> {
        match self {
            Shares::Unset | Shares::Cpu => None,
            Shares::CpuAndMemory { limit } => Some(limit),
        }
    }
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
                && let Some(child) = child.client()
            {
                runs.entry(*owner).or_default().pids.insert(child.id());
            }
        }
        table.jobs.follow(&mut runs);
        Ok(runs)
    }

    /// Enters a member into the table, and a command into its run's job,
    /// marking how the run's memory ceiling holds for it (D95).
    pub(super) fn enrol(&self, id: BacklogId, mut member: Member) -> Result<(), AxError> {
        let mut table = self.hold()?;
        let mark = table.jobs.enter(&member, self.shares, self.affinity);
        if let Body::Command { ceiling, .. } = &mut member.body {
            *ceiling = mark;
        }
        table.members.insert(id, member);
        Ok(())
    }
}

impl Jobs {
    /// How the memory ceiling fared for a command of `owner` that entered
    /// with `mark` and is collected now (D95). The run's refusal count is
    /// read only for a mark that watches it.
    pub(super) fn ceiling(&mut self, owner: RunId, mark: Mark) -> Option<Ceiling> {
        let count = if mark.watching() {
            self.refusals(owner)
        } else {
            None
        };
        super::ceiling::verdict(mark, count)
    }

    #[cfg(windows)]
    fn refusals(&mut self, owner: RunId) -> Option<u64> {
        self.runs.get_mut(&owner)?.refusals()
    }

    #[cfg(not(windows))]
    fn refusals(&mut self, owner: RunId) -> Option<u64> {
        self.cgroups.refusals(owner)
    }

    /// The run's job for a native command, with the shares and the
    /// ceiling's watch set, and the command's ceiling mark read before it
    /// starts. A job that refuses the ceiling or its watch stops the
    /// launch (`NativeWindows.lean` D54).
    #[cfg(windows)]
    pub(super) fn native_job(
        &mut self,
        owner: RunId,
        memory: Option<NonZeroUsize>,
        affinity: RunAffinity,
    ) -> Result<(NonZeroUsize, Mark), AxError> {
        use crate::tools::native_windows::denied;
        let run = self.runs.entry(owner).or_default();
        for retiring in &mut run.retiring {
            retiring.cleanup()?;
        }
        run.retiring.clear();
        if run.job.is_none() {
            run.job = Some(win32job::Job::create().map_err(|err| denied("create run Job", err))?);
        }
        let job = run
            .job
            .as_ref()
            .ok_or_else(|| denied("create run Job", "Job owner disappeared"))?;
        desktop_ffi::cpu::job_share(
            job.handle(),
            RUN_CPU_WEIGHT,
            memory.map_or(0, NonZeroUsize::get),
        )
        .map_err(|err| denied("set run memory ceiling", format!("{err:?}")))?;
        if memory.is_some() && run.watch.is_none() {
            run.watch = Some(
                desktop_ffi::cpu::JobWatch::attach(job.handle())
                    .map_err(|err| denied("watch run memory ceiling", format!("{err:?}")))?,
            );
        }
        if let RunAffinity::Mask(mask) = affinity {
            let mut info = job
                .query_extended_limit_info()
                .map_err(|err| denied("read run Job affinity", err))?;
            info.limit_affinity(mask.get());
            job.set_extended_limit_info(&info)
                .map_err(|err| denied("set run Job affinity", err))?;
        }
        run.share = match memory {
            None => Shares::Cpu,
            Some(memory) => Shares::CpuAndMemory {
                limit: NonZeroU64::new(
                    u64::try_from(memory.get())
                        .map_err(|err| denied("read run Job memory", err))?,
                )
                .ok_or_else(|| denied("read run Job memory", "zero memory"))?,
            },
        };
        run.affinity = affinity;
        let lent = NonZeroUsize::new(
            usize::try_from(job.handle()).map_err(|err| denied("lend run Job", err))?,
        )
        .ok_or_else(|| denied("lend run Job", "invalid Job handle"))?;
        let asked = run.share.memory();
        Ok((lent, run.mark(asked)))
    }

    #[cfg(windows)]
    pub(super) fn keep_failed_native(
        &mut self,
        owner: RunId,
        resources: super::native_windows::RetiringNative,
    ) {
        self.runs.entry(owner).or_default().retiring.push(resources);
    }

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
    fn enter(&mut self, member: &Member, shares: Shares, affinity: RunAffinity) -> Mark {
        let Body::Command {
            child,
            claim: Claim::Window(owner) | Claim::Run(owner),
            ..
        } = &member.body
        else {
            return Mark::NotAsked;
        };
        let Some(child) = child.client() else {
            return Mark::outside(shares.memory(), Unapplied::Unjoined);
        };
        let run = self.runs.entry(*owner).or_default();
        match run.join(child, shares, affinity) {
            Ok(()) => run.mark(shares.memory()),
            Err(why) => {
                run.unjoined = run.unjoined.saturating_add(1);
                Mark::outside(shares.memory(), why)
            }
        }
    }

    #[cfg(not(windows))]
    fn enter(&mut self, member: &Member, shares: Shares, _affinity: RunAffinity) -> Mark {
        let Body::Command {
            child,
            claim: Claim::Window(owner) | Claim::Run(owner),
            ..
        } = &member.body
        else {
            return Mark::NotAsked;
        };
        match child.client() {
            Some(child) => self.cgroups.enter(*owner, child.id(), shares),
            None => Mark::outside(shares.memory(), Unapplied::Unjoined),
        }
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::super::Backlog;
    #[cfg(windows)]
    use super::Shares;

    /// The child reads its own affinity after enrolment, so the assertion
    /// observes the running process rather than just a job's limit record.
    #[cfg(windows)]
    #[test]
    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
            let finished = harvested(backlog, owner);
            std::fs::remove_file(gate).unwrap();
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

    /// A run's command that asks for more than the person's ceiling is
    /// refused there, and its result says the ceiling was hit; a command
    /// of the same run that stays under it says nothing (Exec.lean D95).
    /// The child waits for the gate file, so it allocates only after it
    /// joined the run's job.
    #[cfg(windows)]
    #[test]
    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
    fn a_command_refused_at_the_ceiling_reports_the_hit() {
        let limit = std::num::NonZeroU64::new(512 << 20).unwrap();
        let owner = kernel::RunId::from_bytes([13; 16]);
        let addr = kernel::Address::parse("vault/room1").unwrap();
        let quick = Backlog::with_window(crate::PollBudget::new(1_500, 20))
            .with_shares(Shares::CpuAndMemory { limit });
        let mut small = std::process::Command::new("powershell.exe");
        small.args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"]);
        small.current_dir(std::env::temp_dir());
        let settled = quick
            .run(owner, &addr, "under the ceiling".to_owned(), small)
            .unwrap();
        assert!(
            matches!(
                settled,
                crate::Started::Settled {
                    exit: crate::Exit::Ended { code: 0 },
                    ceiling: None,
                    ..
                }
            ),
            "{settled:?}"
        );
        quick.release(owner);

        let backlog = Backlog::with_window(crate::PollBudget::new(1, 1))
            .with_shares(Shares::CpuAndMemory { limit });
        let gate = backlog
            .scratch
            .dir(backlog.mint().unwrap())
            .with_extension("gate");
        let mut large = std::process::Command::new("powershell.exe");
        large.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "while (-not (Test-Path -LiteralPath $env:D95_GATE)) { Start-Sleep -Milliseconds 20 }; try { $b = New-Object byte[] 1073741824; exit 0 } catch { exit 7 }",
        ]);
        large
            .env("D95_GATE", &gate)
            .current_dir(std::env::temp_dir());
        assert!(matches!(
            backlog
                .run(owner, &addr, "over the ceiling".to_owned(), large)
                .unwrap(),
            crate::Started::Backgrounded { .. }
        ));
        std::fs::write(&gate, b"ready").unwrap();
        let mut finished = None;
        for _ in 0..1_500 {
            if let Some(done) = backlog.harvest(owner).unwrap().into_iter().next() {
                finished = Some(done);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        std::fs::remove_file(gate).unwrap();
        let finished = finished.expect("the allocating command finishes");
        assert_eq!(
            (finished.exit, finished.ceiling),
            (
                crate::Exit::Ended { code: 7 },
                Some(crate::Ceiling::Hit { limit })
            ),
            "{}",
            finished.stderr
        );
        backlog.release(owner);
    }

    /// The same value seam compiles elsewhere without claiming affinity.
    #[cfg(not(windows))]
    #[test]
    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
        let done = harvested(&backlog, owner);
        backlog.release(owner);
        assert_eq!(
            done.exit,
            crate::Exit::Ended { code: 23 },
            "{}",
            done.stderr
        );
    }

    /// The first command of `owner` to finish. It waits on that condition
    /// alone: how long `powershell.exe` takes to start depends on how busy
    /// the machine is, so a counted budget fails a correct command on a
    /// loaded runner, and a command that never ends is ended by nextest's
    /// `terminate-after` instead.
    #[cfg(windows)]
    fn harvested(backlog: &Backlog, owner: kernel::RunId) -> crate::Finished {
        loop {
            if let Some(done) = backlog.harvest(owner).unwrap().into_iter().next() {
                return done;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
