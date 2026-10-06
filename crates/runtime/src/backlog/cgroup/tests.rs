// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]

//! What a run reads back from a cgroup root: nothing where the root may
//! not be written, and the weight and the limit the arm asked for where
//! it may (`crates/runtime/spec/Tools/Exec.lean` D29, D33).
//!
//! Every test builds a fake `/sys/fs/cgroup` under the temp directory,
//! so the same checks run on the machine the code is written on and on
//! the one it ships to.

use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{Cgroups, RUN_CPU_WEIGHT};
use crate::Shares;

/// How many fake roots this process has made.
static ROOTS: AtomicU64 = AtomicU64::new(0);

/// A fake `/sys/fs/cgroup`, removed when it drops.
struct FakeRoot(PathBuf);

impl FakeRoot {
    /// One whose parent cgroup may be written: the two files the probe
    /// opens exist, as the kernel gives them.
    fn delegated() -> FakeRoot {
        let root = FakeRoot::fresh();
        for name in ["cgroup.procs", "cgroup.subtree_control"] {
            std::fs::write(root.path().join(name), "").unwrap();
        }
        root
    }

    /// One that may not be written: no cgroup files at all.
    fn plain() -> FakeRoot {
        FakeRoot::fresh()
    }

    fn fresh() -> FakeRoot {
        let path = std::env::temp_dir().join(format!(
            "sprawling-cgroup-{}-{}",
            std::process::id(),
            ROOTS.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        FakeRoot(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// One file of the fake root as a whole string.
    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.0.join(relative)).unwrap()
    }

    fn has(&self, relative: &str) -> bool {
        self.0.join(relative).exists()
    }
}

impl Drop for FakeRoot {
    fn drop(&mut self) {
        if let Err(kept) = std::fs::remove_dir_all(&self.0) {
            eprintln!(
                "the fake cgroup root stayed at {}: {kept}",
                self.0.display()
            );
        }
    }
}

/// Where the cgroup may not be written, a run reads as unshared and
/// nothing is made.
#[test]
fn a_run_reads_unshared_where_the_root_cannot_be_written() {
    let root = FakeRoot::plain();
    let run = kernel::RunId::from_bytes([7; 16]);
    let asked = Shares::CpuAndMemory {
        limit: NonZeroU64::new(8 << 30).unwrap(),
    };
    let mut cgroups = Cgroups::adopt(root.path(), 4242);
    assert_eq!(cgroups.enter(run, 111, asked), Shares::Unset);
    assert_eq!(cgroups.held(run), Shares::Unset);
    assert!(!root.has("core"), "the harness must not be moved");
    assert!(
        !root.has(&format!("run-{run}")),
        "no run cgroup may be made"
    );
}

/// Two runs get the same weight, `memory.max` only where the arm asked
/// for a limit, and each command's pid joins its own run's cgroup.
#[test]
fn two_runs_hold_the_same_weight_and_the_limit_the_arm_asked_for() {
    let root = FakeRoot::delegated();
    let limit = NonZeroU64::new(8 << 30).unwrap();
    let asked = Shares::CpuAndMemory { limit };
    let first = kernel::RunId::from_bytes([7; 16]);
    let second = kernel::RunId::from_bytes([8; 16]);
    let mut cgroups = Cgroups::adopt(root.path(), 4242);

    assert_eq!(cgroups.enter(first, 111, asked), asked);
    assert_eq!(cgroups.enter(second, 222, Shares::Cpu), Shares::Cpu);
    assert_eq!(cgroups.held(first), asked);
    assert_eq!(cgroups.held(second), Shares::Cpu);

    assert_eq!(root.read("core/cgroup.procs"), "4242");
    assert_eq!(root.read("cgroup.subtree_control"), "+cpu +memory");
    assert_eq!(
        root.read(&format!("run-{first}/cpu.weight")),
        RUN_CPU_WEIGHT.to_string()
    );
    assert_eq!(
        root.read(&format!("run-{second}/cpu.weight")),
        RUN_CPU_WEIGHT.to_string()
    );
    assert_eq!(
        root.read(&format!("run-{first}/memory.max")),
        limit.get().to_string()
    );
    assert!(!root.has(&format!("run-{second}/memory.max")));
    assert_eq!(root.read(&format!("run-{first}/cgroup.procs")), "111");
    assert_eq!(root.read(&format!("run-{second}/cgroup.procs")), "222");

    cgroups.forget(first);
    assert_eq!(cgroups.held(first), Shares::Unset);
}

/// A restarted harness can reuse a run directory after its user clears the ceiling.
#[test]
fn cpu_only_clears_a_previous_ceiling_when_a_run_directory_is_reused() {
    let root = FakeRoot::delegated();
    let run = kernel::RunId::from_bytes([9; 16]);
    let mut first = Cgroups::adopt(root.path(), 4242);
    let capped = Shares::CpuAndMemory {
        limit: NonZeroU64::new(64 << 20).unwrap(),
    };
    assert_eq!(first.enter(run, 111, capped), capped);
    let mut restarted = Cgroups::adopt(root.path(), 4242);
    let held = restarted.enter(run, 222, Shares::Cpu);
    assert_eq!(
        (held, root.read(&format!("run-{run}/memory.max"))),
        (Shares::Cpu, "max".to_owned())
    );
}
