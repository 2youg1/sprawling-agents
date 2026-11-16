// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Guarantee {
    Filesystem,

    Network,

    ProcessTree,

    User,

    Resources,
}

impl Guarantee {
    pub const ALL: [Guarantee; 5] = [
        Guarantee::Filesystem,
        Guarantee::Network,
        Guarantee::ProcessTree,
        Guarantee::User,
        Guarantee::Resources,
    ];

    pub fn phrase(self) -> &'static str {
        match self {
            Guarantee::Filesystem => "writes through the working directory land on a copy",
            Guarantee::Network => "the network is closed",
            Guarantee::ProcessTree => "the whole process tree ends with the command",
            Guarantee::User => "the command has an isolated security identity",
            Guarantee::Resources => "CPU and memory ceilings are enforced",
        }
    }

    pub fn unkept(self) -> &'static str {
        match self {
            Guarantee::Filesystem => "that writes land on a copy",
            Guarantee::Network => "network isolation",
            Guarantee::ProcessTree => "process-tree containment",
            Guarantee::User => "an isolated security identity",
            Guarantee::Resources => "CPU and memory ceilings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assurances {
    pub filesystem: Kept,
    pub network: Kept,
    pub process_tree: Kept,
    pub user: Kept,
    pub resources: Kept,
}

impl Assurances {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    ScratchDirectory,
}

impl Missing {
    pub fn phrase(self) -> &'static str {
        match self {
            Missing::ScratchDirectory => "a writable scratch directory",
        }
    }

    pub fn recovery(self) -> &'static str {
        match self {
            Missing::ScratchDirectory => {
                "point TMPDIR (TEMP on Windows) at a directory this process can write, \
                 and start sprawling again"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confinement {
    LinuxNamespaces { wrapper: PathBuf },

    WindowsJobObject,

    MacosSeatbelt { wrapper: PathBuf },

    CopiedTree,

    Unavailable { missing: Missing },
}

impl Confinement {
    pub fn detect() -> Confinement {
        Confinement::choose(&Offerings::this_machine())
    }

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
            Confinement::MacosSeatbelt { .. } => Assurances {
                filesystem: all,
                network: all,
                process_tree: none,
                user: none,
                resources: none,
            },
            Confinement::WindowsJobObject => Assurances {
                filesystem: all,
                network: all,
                process_tree: all,
                user: all,
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

    pub fn name(&self) -> &'static str {
        match self {
            Confinement::LinuxNamespaces { .. } => "linux_namespaces",
            Confinement::WindowsJobObject => "windows_job_object",
            Confinement::MacosSeatbelt { .. } => "macos_seatbelt",
            Confinement::CopiedTree => "copied_tree",
            Confinement::Unavailable { .. } => "unavailable",
        }
    }

    pub fn statement(&self) -> String {
        match self {
            Confinement::Unavailable { missing } => {
                format!("no sandbox is available: {}", missing.phrase())
            }
            Confinement::LinuxNamespaces { .. }
            | Confinement::WindowsJobObject
            | Confinement::MacosSeatbelt { .. }
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
                let statement = format!(
                    "{}: it guarantees that {}; it does not guarantee: {}.",
                    self.name(),
                    listed(&holds),
                    listed(&misses)
                );
                #[cfg(target_os = "linux")]
                if matches!(self, Confinement::LinuxNamespaces { .. }) {
                    return format!(
                        "{statement} An unprivileged user namespace is required; \
                         namespace setup failure refuses the placement."
                    );
                }
                statement
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offerings {
    pub namespace_tool: Option<PathBuf>,

    pub scratch: Option<PathBuf>,
}

impl Offerings {
    pub fn this_machine() -> Offerings {
        let scratch = std::env::temp_dir();
        Offerings {
            namespace_tool: namespace_wrapper(),
            scratch: scratch.is_dir().then_some(scratch),
        }
    }
}

fn namespace_wrapper() -> Option<PathBuf> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("bwrap"))
        .find(|candidate| candidate.is_file())
}

fn listed(parts: &[&str]) -> String {
    match parts {
        [] => "nothing".to_owned(),
        [one] => (*one).to_owned(),
        [one, two] => format!("{one} and {two}"),
        [first @ .., last] => format!("{}, and {last}", first.join(", ")),
    }
}

mod placement;
mod placing;

pub use placement::{Placement, parse_placement};
pub use placing::{Confined, Placed};

impl Confined {
    pub(super) fn prepare(
        &mut self,
        command: std::process::Command,
        workdir: &std::path::Path,
        shares: crate::backlog::Shares,
    ) -> Result<(std::process::Command, Placed), kernel::AxError> {
        #[cfg(target_os = "macos")]
        if matches!(self.arm(), Confinement::MacosSeatbelt { .. }) {
            super::yielding::require_executable(&command)?;
            let (command, placed) = self.place(command, workdir)?;
            return match super::yielding::one_level_down(command, shares) {
                Ok(command) => Ok((command, placed)),
                Err(fault) => {
                    self.settled(placed);
                    Err(fault)
                }
            };
        }
        self.place(super::yielding::one_level_down(command, shares)?, workdir)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
