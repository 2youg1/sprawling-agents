// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The confinement a host command runs under: which arm this machine
//! gives, and what that arm promises.
//!
//! [`crate::sandbox`] is the other half of the execution boundary: a
//! wasip1 guest that reaches a host directory only when a
//! [`Mount`](crate::sandbox::Mount) named it, and reaches no network at
//! all because wasip1 has no way to obtain a socket. A program the agent
//! asks for is not a guest, so its isolation has to be bought from the
//! platform, and no platform sells all of it. What this module owns is
//! therefore a taxonomy rather than a switch: every arm states, axis by
//! axis, what it keeps and what it does not.
//!
//! **An arm that cannot say what it does not hold is worse than no arm.**
//! The agent reads [`Confinement::statement`] in the exec tool's own
//! description before it acts; a report that said only "sandboxed" would
//! send it into a box whose network is open.
//!
//! A placement is a copy of the working tree, made one command at a
//! time, so a command that ruins it ruins nothing of the person's. The
//! copy is bounded and lives in
//! [`placing::copy`](crate::tools::Confinement); a tree over the bound is
//! refused by name rather than half-copied, because a sandbox that
//! silently answers for files it did not carry is the failure this
//! module exists to prevent.

use std::path::PathBuf;

/// One axis a confinement arm can promise, or refuse to promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Guarantee {
    /// Writes through the working directory land on a copy; the tree the
    /// person has stays as it is.
    Filesystem,
    /// The command cannot open a network connection.
    Network,
    /// The whole process tree ends when the command ends.
    ProcessTree,
    /// The command runs with an identity of its own.
    User,
    /// CPU and memory ceilings are enforced, not merely asked for.
    Resources,
}

impl Guarantee {
    /// Every axis, in the order a report lists them.
    pub const ALL: [Guarantee; 5] = [
        Guarantee::Filesystem,
        Guarantee::Network,
        Guarantee::ProcessTree,
        Guarantee::User,
        Guarantee::Resources,
    ];

    /// What holding this axis means, in the words a sentence shown to a
    /// caller may use.
    pub fn phrase(self) -> &'static str {
        match self {
            Guarantee::Filesystem => "writes through the working directory land on a copy",
            Guarantee::Network => "the network is closed",
            Guarantee::ProcessTree => "the whole process tree ends with the command",
            Guarantee::User => "the command runs as a user of its own",
            Guarantee::Resources => "CPU and memory ceilings are enforced",
        }
    }

    /// What not holding it means, as the thing the caller does not get.
    /// Stated as a missing capability rather than as an absence, because
    /// the list it lands in is a list of what is not bought.
    pub fn unkept(self) -> &'static str {
        match self {
            Guarantee::Filesystem => "that writes land on a copy",
            Guarantee::Network => "network isolation",
            Guarantee::ProcessTree => "process-tree containment",
            Guarantee::User => "a user of its own",
            Guarantee::Resources => "CPU and memory ceilings",
        }
    }
}

/// Whether one arm keeps one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    Yes,
    No,
}

/// What one arm promises, axis by axis.
///
/// Every arm states all five axes, and that is the point of the type: a
/// new axis is a compile error in every arm's constructor, so no arm can
/// keep a stale answer to a question that arrived after it was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assurances {
    pub filesystem: Kept,
    pub network: Kept,
    pub process_tree: Kept,
    pub user: Kept,
    pub resources: Kept,
}

impl Assurances {
    /// The whole list, in the order [`Guarantee::ALL`] declares.
    pub fn of(self, axis: Guarantee) -> Kept {
        match axis {
            Guarantee::Filesystem => self.filesystem,
            Guarantee::Network => self.network,
            Guarantee::ProcessTree => self.process_tree,
            Guarantee::User => self.user,
            Guarantee::Resources => self.resources,
        }
    }
}

/// What a machine lacks when none of its arms can take a command at all.
///
/// A name, never a sentence: a page supplies the words, and a machine
/// that lacks something else gets an arm here rather than a second
/// string field no reader can match on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// A writable directory a working tree can be copied into.
    ScratchDirectory,
}

impl Missing {
    /// The thing itself, for a refusal's subject.
    pub fn phrase(self) -> &'static str {
        match self {
            Missing::ScratchDirectory => "a writable scratch directory",
        }
    }

    /// What a person does about it.
    pub fn recovery(self) -> &'static str {
        match self {
            Missing::ScratchDirectory => {
                "point TMPDIR (TEMP on Windows) at a directory this process can write, \
                 and start sprawling again"
            }
        }
    }
}

/// The backends, by platform, each carrying what it promises.
///
/// `WindowsJobObject` is the arm a Windows machine with a job-object
/// enforcer compiled in would report. No build of this crate constructs
/// one, because `CreateJobObject` is a foreign call the workspace
/// forbids outside `desktop/`; a Windows machine therefore reports
/// [`Confinement::CopiedTree`], whose [`Assurances`] say what it does
/// not hold (the network, above all) rather than implying it holds
/// everything a job object would.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confinement {
    /// Namespaces of its own — mount, network, process, user — made by a
    /// wrapper program on this machine.
    LinuxNamespaces { wrapper: PathBuf },
    /// A Windows job object: the tree ends together, the limits hold,
    /// and the network is not isolated.
    WindowsJobObject,
    /// The floor every platform has: the command runs in a copy of the
    /// working tree, and the tree itself is not what it writes to.
    CopiedTree,
    /// No arm at all, and what this machine is missing.
    Unavailable { missing: Missing },
}

impl Confinement {
    /// The arm this machine gives a command. One sampling point: the
    /// scratch root, the search path and this platform are asked here
    /// and nowhere else, so a report and a placement cannot disagree.
    pub fn detect() -> Confinement {
        Confinement::choose(&Offerings::this_machine())
    }

    /// The arm a machine with these capabilities gives.
    pub fn choose(offered: &Offerings) -> Confinement {
        if offered.scratch.is_none() {
            return Confinement::Unavailable {
                missing: Missing::ScratchDirectory,
            };
        }
        match &offered.namespace_tool {
            Some(wrapper) => Confinement::LinuxNamespaces {
                wrapper: wrapper.clone(),
            },
            None => Confinement::CopiedTree,
        }
    }

    /// What this arm promises. A pure function of the arm, so a report
    /// can be read without running anything.
    pub fn assurances(&self) -> Assurances {
        let all = Kept::Yes;
        let none = Kept::No;
        match self {
            Confinement::LinuxNamespaces { .. } => Assurances {
                filesystem: all,
                network: all,
                process_tree: all,
                user: all,
                resources: none,
            },
            Confinement::WindowsJobObject => Assurances {
                filesystem: all,
                network: none,
                process_tree: all,
                user: none,
                resources: all,
            },
            Confinement::CopiedTree => Assurances {
                filesystem: all,
                network: none,
                process_tree: none,
                user: none,
                resources: none,
            },
            Confinement::Unavailable { .. } => Assurances {
                filesystem: none,
                network: none,
                process_tree: none,
                user: none,
                resources: none,
            },
        }
    }

    /// The name a report carries. Closed and spelled once, because a
    /// page, a refusal and a test all have to mean the same arm.
    pub fn name(&self) -> &'static str {
        match self {
            Confinement::LinuxNamespaces { .. } => "linux_namespaces",
            Confinement::WindowsJobObject => "windows_job_object",
            Confinement::CopiedTree => "copied_tree",
            Confinement::Unavailable { .. } => "unavailable",
        }
    }

    /// What a command under this arm can and cannot count on, in one
    /// sentence. Derived from [`Confinement::assurances`] rather than
    /// written beside it, so the sentence cannot drift from the type.
    pub fn statement(&self) -> String {
        match self {
            Confinement::Unavailable { missing } => {
                format!("no sandbox is available: {}", missing.phrase())
            }
            Confinement::LinuxNamespaces { .. }
            | Confinement::WindowsJobObject
            | Confinement::CopiedTree => {
                let kept = self.assurances();
                let holds: Vec<&str> = Guarantee::ALL
                    .iter()
                    .filter(|axis| kept.of(**axis) == Kept::Yes)
                    .map(|axis| axis.phrase())
                    .collect();
                let misses: Vec<&str> = Guarantee::ALL
                    .iter()
                    .filter(|axis| kept.of(**axis) == Kept::No)
                    .map(|axis| axis.unkept())
                    .collect();
                format!(
                    "{}: it guarantees that {}; it does not guarantee: {}.",
                    self.name(),
                    listed(&holds),
                    listed(&misses)
                )
            }
        }
    }
}

/// What a machine has, read once and handed to [`Confinement::choose`]
/// so that a test states a machine instead of borrowing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offerings {
    /// The program that makes this machine's own isolation, where one
    /// exists.
    pub namespace_tool: Option<PathBuf>,
    /// A writable directory a working tree can be copied into.
    pub scratch: Option<PathBuf>,
}

impl Offerings {
    /// Reads this machine: whether the platform's wrapper program is on
    /// the search path, and whether the scratch root exists.
    pub fn this_machine() -> Offerings {
        let scratch = std::env::temp_dir();
        Offerings {
            namespace_tool: namespace_wrapper(),
            scratch: scratch.is_dir().then_some(scratch),
        }
    }
}

/// The wrapper that gives a Linux command namespaces of its own.
///
/// One program, looked up by its exact name, because the arm it makes
/// exists on Linux alone: a search path walked here is not a second
/// authority on which programs a machine has — the doctor's requirement
/// table is that — it is the question of whether *this* arm can be
/// given at all.
fn namespace_wrapper() -> Option<PathBuf> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("bwrap"))
        .find(|candidate| candidate.is_file())
}

/// A list in the shape a sentence uses: `a`, `a and b`, `a, b and c`.
fn listed(parts: &[&str]) -> String {
    match parts {
        [] => "nothing".to_owned(),
        [one] => (*one).to_owned(),
        [one, two] => format!("{one} and {two}"),
        [first @ .., last] => format!("{}, and {last}", first.join(", ")),
    }
}

mod placing;

pub use placing::{Confined, Placed, Placement, parse_placement};

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
