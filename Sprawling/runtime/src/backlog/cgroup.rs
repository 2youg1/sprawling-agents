// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Each Linux run's child cgroup: the weight every run holds, and the
//! memory limit when the arm asked for one (`crates/runtime/spec/Tools/Exec.lean` D29, D33).
//!
//! cgroup v2 is a filesystem, so this module is plain file reads and
//! writes: the harness moves itself into `core` under its own cgroup —
//! a cgroup that holds processes may not hand out its own resources —
//! opens `cpu` and `memory` in the parent's `cgroup.subtree_control`,
//! and gives each run a child named `run-<RunId>`. The child holds
//! `cpu.weight` 100, the same for every run, and `memory.max` set to the
//! requested limit or `max` to clear a limit in a reused directory; each command of the run writes its pid into
//! the child's `cgroup.procs`.
//!
//! The parent is a parameter rather than a constant, so the tests run
//! a whole fake `/sys/fs/cgroup` under the temp directory on any
//! platform, and this file says nothing about what a real machine's
//! cgroup happens to be. [`platform_shares`] is the one place the
//! doctor and the wiring read whether this machine's cgroup is
//! delegated, and a machine that has none, or may not write it, sets
//! nothing and says so instead.

#[cfg(any(not(windows), test))]
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(any(not(windows), test))]
use std::sync::OnceLock;

#[cfg(any(not(windows), test))]
use kernel::RunId;

#[cfg(any(not(windows), test))]
use super::Shares;
#[cfg(any(not(windows), test))]
use super::ceiling::{Mark, Unapplied};

/// The weight every run's child cgroup holds, from 1 to 10000: the same
/// for every run, so the runs share the processors evenly among
/// themselves (`crates/runtime/spec/Tools/Exec.lean` D29).
#[cfg(any(not(windows), test))]
const RUN_CPU_WEIGHT: u64 = 100;

/// Which halves of a run's shares this machine can set (D29).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformShares {
    /// Nothing can be set here.
    None,
    /// The CPU half alone: macOS's `taskpolicy`.
    Cpu,
    /// Both halves: a Windows job, or a Linux cgroup this process may
    /// write.
    CpuAndMemory,
}

/// What this machine can give a run of what an arm asks for (D29, D33).
///
/// Windows sets both halves on the run's job, macOS the CPU half
/// through `taskpolicy`, and Linux sets both where the harness's own
/// cgroup may be written and nothing where it may not. The doctor and
/// the backlog read this one answer, so the two cannot tell a machine
/// different things.
#[must_use]
pub fn platform_shares() -> PlatformShares {
    if cfg!(windows) {
        PlatformShares::CpuAndMemory
    } else if cfg!(target_os = "macos") {
        PlatformShares::Cpu
    } else {
        match own_cgroup() {
            Some(parent) if writable(&parent) => PlatformShares::CpuAndMemory,
            _ => PlatformShares::None,
        }
    }
}

/// Each run's child cgroup, made under the harness's own (D29, D33).
///
/// No Windows run ever holds one — [`own_cgroup`] answers `None` there —
/// but the tests drive the same type against a fake root on every
/// platform, so it exists wherever a test build compiles.
#[cfg(any(not(windows), test))]
#[derive(Default)]
pub(super) struct Cgroups {
    /// The parent the runs' children are made under, once it was found
    /// writable; `None` where this machine has none.
    parent: Option<PathBuf>,
    /// Set once adoption was attempted, so a machine without a
    /// delegated cgroup asks once rather than once per command.
    adopted: bool,
    /// The shares each run holds.
    runs: BTreeMap<RunId, Shares>,
}

#[cfg(any(not(windows), test))]
impl Cgroups {
    /// Adopts `parent`: the harness moves itself into `parent/core` and
    /// `parent/cgroup.subtree_control` opens `cpu` and `memory`, so the
    /// runs' children can hold both. A `parent` that may not be written
    /// adopts nothing, and its runs read as unshared (D33).
    ///
    /// The tests' door onto the same adoption: a real machine reaches it
    /// through [`Cgroups::this_machine`] and its own delegated parent, so
    /// this takes the root a test lent it, which no build outside a test
    /// has a caller for.
    #[cfg(test)]
    pub(super) fn adopt(parent: &Path, pid: u32) -> Cgroups {
        let mut cgroups = Cgroups {
            parent: None,
            adopted: true,
            runs: BTreeMap::new(),
        };
        if writable(parent) && core(parent, pid).is_ok() {
            cgroups.parent = Some(parent.to_path_buf());
        }
        cgroups
    }

    /// The cgroup this process is in on this machine, adopted on first
    /// use (D33).
    fn this_machine() -> Cgroups {
        Cgroups {
            parent: adopted(),
            adopted: true,
            runs: BTreeMap::new(),
        }
    }

    /// Puts `pid` into `run`'s child cgroup, making the child on first
    /// use with the shares `asked` for, and answers how the memory
    /// ceiling `asked` holds for this command (Exec.lean D95). A run whose
    /// cgroup refused its shares holds `Unset` and its command starts
    /// anyway (D29).
    pub(super) fn enter(&mut self, run: RunId, pid: u32, asked: Shares) -> Mark {
        let limit = asked.memory();
        if asked == Shares::Unset {
            return Mark::NotAsked;
        }
        if !self.adopted {
            *self = Cgroups::this_machine();
        }
        let Some(parent) = &self.parent else {
            let why = if cfg!(target_os = "macos") {
                Unapplied::Platform
            } else {
                Unapplied::NotDelegated
            };
            return Mark::outside(limit, why);
        };
        let dir = parent.join(format!("run-{run}"));
        if let std::collections::btree_map::Entry::Vacant(first) = self.runs.entry(run) {
            if !made(&dir, asked) {
                return Mark::outside(limit, Unapplied::Refused);
            }
            first.insert(asked);
        }
        // A command whose pid cannot be written still runs, under its
        // parent's shares and outside the ceiling, and its result says so.
        if std::fs::write(dir.join("cgroup.procs"), pid.to_string()).is_err() {
            return Mark::outside(limit, Unapplied::Unjoined);
        }
        match limit {
            Some(limit) => Mark::entered(limit, refusals(&dir)),
            None => Mark::NotAsked,
        }
    }

    /// How many times `run`'s cgroup refused an allocation at its
    /// `memory.max`; `None` when it has no cgroup or the count does not
    /// read (Exec.lean D95).
    pub(super) fn refusals(&self, run: RunId) -> Option<u64> {
        refusals(&self.parent.as_ref()?.join(format!("run-{run}")))
    }

    /// The shares `run` holds; `Unset` for a run this machine never
    /// gave a child cgroup.
    pub(super) fn held(&self, run: RunId) -> Shares {
        self.runs.get(&run).copied().unwrap_or_default()
    }

    /// Lets the run's record go; its cgroup directory stays, because a
    /// cgroup that holds a process cannot be removed (D33).
    pub(super) fn forget(&mut self, run: RunId) {
        self.runs.remove(&run);
    }
}

/// Makes the run's child cgroup under `parent` and writes the shares
/// `asked` for: `cpu.weight` for every run, and `memory.max` set to the
/// requested limit or `max` to clear a prior ceiling. `false` when a write was refused, which leaves the run
/// unshared rather than failing its command (D29).
#[cfg(any(not(windows), test))]
fn made(dir: &Path, asked: Shares) -> bool {
    let weighed = std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(dir.join("cpu.weight"), RUN_CPU_WEIGHT.to_string()));
    match asked {
        Shares::CpuAndMemory { limit } => weighed
            .and_then(|()| std::fs::write(dir.join("memory.max"), limit.get().to_string()))
            .is_ok(),
        Shares::Cpu | Shares::Unset => weighed
            .and_then(|()| std::fs::write(dir.join("memory.max"), "max"))
            .is_ok(),
    }
}

/// The `oom` line of a run cgroup's `memory.events`: how many times its
/// usage reached `memory.max` and an allocation was about to fail. The
/// `max` line is not read, because it also counts a reclaim that
/// succeeded (Exec.lean D95).
#[cfg(any(not(windows), test))]
fn refusals(dir: &Path) -> Option<u64> {
    std::fs::read_to_string(dir.join("memory.events"))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("oom "))?
        .trim()
        .parse()
        .ok()
}

/// Moves this process into `parent/core` and opens `cpu` and `memory`
/// for the runs' children: a cgroup that holds processes may not enable
/// its own controllers, so the harness leaves the parent first (D29).
#[cfg(any(not(windows), test))]
fn core(parent: &Path, pid: u32) -> std::io::Result<()> {
    let inner = parent.join("core");
    std::fs::create_dir_all(&inner)?;
    std::fs::write(inner.join("cgroup.procs"), pid.to_string())?;
    std::fs::write(parent.join("cgroup.subtree_control"), "+cpu +memory")
}

/// The parent the runs' cgroups of this process are made under: the
/// harness's own cgroup, adopted once on first use, or `None` where
/// this machine has none it may write (D33).
#[cfg(any(not(windows), test))]
fn adopted() -> Option<PathBuf> {
    static PARENT: OnceLock<Option<PathBuf>> = OnceLock::new();
    PARENT
        .get_or_init(|| {
            let parent = own_cgroup()?;
            if !writable(&parent) {
                return None;
            }
            core(&parent, std::process::id()).ok()?;
            Some(parent)
        })
        .clone()
}

/// This process's own cgroup on this machine: the `0::` line of
/// `/proc/self/cgroup`, under the cgroup v2 mount point. `None` where
/// the platform has no such file or no such line (D29).
fn own_cgroup() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let raw = std::fs::read_to_string("/proc/self/cgroup").ok()?;
        let relative = raw.lines().find_map(|line| line.strip_prefix("0::"))?;
        Some(Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/')))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Whether `parent` accepts the writes adoption makes: both files are
/// opened for writing and closed, which changes nothing (D33).
fn writable(parent: &Path) -> bool {
    ["cgroup.procs", "cgroup.subtree_control"]
        .iter()
        .all(|name| {
            std::fs::OpenOptions::new()
                .write(true)
                .open(parent.join(name))
                .is_ok()
        })
}

#[cfg(test)]
mod tests;
