// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

#[cfg(any(not(windows), test))]
const RUN_CPU_WEIGHT: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformShares {

    None,

    Cpu,

    CpuAndMemory,
}

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

#[cfg(any(not(windows), test))]
#[derive(Default)]
pub(super) struct Cgroups {

    parent: Option<PathBuf>,

    adopted: bool,

    runs: BTreeMap<RunId, Shares>,
}

#[cfg(any(not(windows), test))]
impl Cgroups {

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

    fn this_machine() -> Cgroups {
        Cgroups {
            parent: adopted(),
            adopted: true,
            runs: BTreeMap::new(),
        }
    }

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

        if std::fs::write(dir.join("cgroup.procs"), pid.to_string()).is_err() {
            return Mark::outside(limit, Unapplied::Unjoined);
        }
        match limit {
            Some(limit) => Mark::entered(limit, refusals(&dir)),
            None => Mark::NotAsked,
        }
    }

    pub(super) fn refusals(&self, run: RunId) -> Option<u64> {
        refusals(&self.parent.as_ref()?.join(format!("run-{run}")))
    }

    pub(super) fn held(&self, run: RunId) -> Shares {
        self.runs.get(&run).copied().unwrap_or_default()
    }

    pub(super) fn forget(&mut self, run: RunId) {
        self.runs.remove(&run);
    }
}

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

#[cfg(any(not(windows), test))]
fn core(parent: &Path, pid: u32) -> std::io::Result<()> {
    let inner = parent.join("core");
    std::fs::create_dir_all(&inner)?;
    std::fs::write(inner.join("cgroup.procs"), pid.to_string())?;
    std::fs::write(parent.join("cgroup.subtree_control"), "+cpu +memory")
}

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
