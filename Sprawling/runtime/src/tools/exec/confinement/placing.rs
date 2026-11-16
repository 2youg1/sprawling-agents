// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use kernel::{AxCode, AxError};

use crate::backlog::{BacklogId, Finished};

use super::{Confinement, Missing, Offerings};

pub struct Confined {
    arm: Confinement,
    scratch: Option<PathBuf>,
    idle: Option<Mirror>,
    outstanding: BTreeMap<BacklogId, Mirror>,
}

struct Mirror {
    of: PathBuf,
    at: PathBuf,
}

pub struct Placed {
    copy: Option<Mirror>,
    work: storage::FileWork,
}

impl Placed {
    pub(in crate::tools::exec) fn into_container_copy(self) -> Result<PathBuf, AxError> {
        self.copy
            .map(|copy| copy.at)
            .ok_or_else(|| no_arm(Missing::ScratchDirectory))
    }

    pub fn work(&self) -> storage::FileWork {
        self.work
    }
}

impl Confined {

    pub fn detect() -> Confined {
        let offered = Offerings::this_machine();
        Confined {
            arm: Confinement::choose(&offered),
            scratch: offered.scratch,
            idle: None,
            outstanding: BTreeMap::new(),
        }
    }

    pub fn with_arm(arm: Confinement, scratch: Option<PathBuf>) -> Confined {
        Confined {
            arm,
            scratch,
            idle: None,
            outstanding: BTreeMap::new(),
        }
    }

    pub fn arm(&self) -> &Confinement {
        &self.arm
    }

    pub fn statement(&self) -> String {
        self.arm.statement()
    }

    pub fn place(
        &mut self,
        mut command: Command,
        workdir: &Path,
    ) -> Result<(Command, Placed), AxError> {
        let wrapper = match &self.arm {
            Confinement::Unavailable { missing } => return Err(no_arm(*missing)),
            Confinement::WindowsJobObject => {
                #[cfg(not(windows))]
                return Err(no_job_object());
                #[cfg(windows)]
                {
                    None
                }
            }
            Confinement::MacosSeatbelt { .. } => {
                #[cfg(target_os = "macos")]
                {
                    None
                }
                #[cfg(not(target_os = "macos"))]
                {
                    return Err(AxError::failure(
                        AxCode::SandboxDenied,
                        "initialize macOS Seatbelt confinement",
                        "this host is not macOS",
                    )
                    .with_recovery("select a native sandbox for this host platform"));
                }
            }
            Confinement::LinuxNamespaces { wrapper } => {
                #[cfg(target_os = "linux")]
                {
                    let denied = |detail| {
                        AxError::failure(
                            AxCode::SandboxDenied,
                            "create the Linux confinement",
                            format!(
                                "this arm requires an unprivileged user namespace; \
                                 namespace setup failed: {detail}"
                            ),
                        )
                        .with_recovery(
                            "use a Linux machine that permits unprivileged user namespaces, \
                             or ask the User to choose `where: host`",
                        )
                    };
                    let probe = namespaced(wrapper, workdir, workdir, &child::command("/bin/true"))
                        .output()
                        .map_err(|err| denied(err.to_string()))?;
                    if !probe.status.success() {
                        return Err(denied(format!(
                            "{}: {}",
                            probe.status,
                            String::from_utf8_lossy(&probe.stderr).trim()
                        )));
                    }
                }
                Some(wrapper.clone())
            }
            Confinement::CopiedTree => None,
        };
        let (copy, work) = self.synced(workdir)?;
        #[cfg(target_os = "macos")]
        if let Confinement::MacosSeatbelt { wrapper } = &self.arm {
            let native = super::super::native_macos::probe(wrapper, &copy.at)
                .and_then(|()| super::super::native_macos::wrap(wrapper, &copy.at, &command));
            return match native {
                Ok(command) => Ok((
                    command,
                    Placed {
                        copy: Some(copy),
                        work,
                    },
                )),
                Err(error) => {
                    remove(&copy.at);
                    Err(error)
                }
            };
        }
        let command = match wrapper {
            Some(wrapper) => namespaced(&wrapper, &copy.at, workdir, &command),
            None => {
                command.current_dir(&copy.at);
                command
            }
        };
        Ok((
            command,
            Placed {
                copy: Some(copy),
                work,
            },
        ))
    }

    pub fn settled(&mut self, placed: Placed) {
        if let Some(copy) = placed.copy {
            self.keep(copy);
        }
    }

    pub fn handed(&mut self, id: BacklogId, placed: Placed) {
        if let Some(copy) = placed.copy {
            self.outstanding.insert(id, copy);
        }
    }

    pub fn reaped(&mut self, finished: &[Finished]) {
        for member in finished {
            if let Some(copy) = self.outstanding.remove(&member.id) {
                self.keep(copy);
            }
        }
    }

    fn keep(&mut self, copy: Mirror) {
        match self.idle {
            None => self.idle = Some(copy),
            Some(_) => remove(&copy.at),
        }
    }

    fn synced(&mut self, workdir: &Path) -> Result<(Mirror, storage::FileWork), AxError> {
        let root = self
            .scratch
            .as_deref()
            .ok_or_else(|| no_arm(Missing::ScratchDirectory))?;
        let copy = match self.idle.take() {
            Some(kept) if kept.of == workdir => kept,
            Some(other) => {
                remove(&other.at);
                Mirror {
                    of: workdir.to_path_buf(),
                    at: fresh(root)?,
                }
            }
            None => Mirror {
                of: workdir.to_path_buf(),
                at: fresh(root)?,
            },
        };
        let mut budget = Budget::new();
        match mirror(
            &Stage {
                from: workdir,
                into: &copy.at,
                copy: &copy.at,
            },
            0,
            &mut budget,
        ) {
            Ok(()) => Ok((copy, budget.work())),
            Err(fault) => {

                remove(&copy.at);
                Err(fault)
            }
        }
    }
}

impl Drop for Confined {
    fn drop(&mut self) {

        for copy in self.idle.iter().chain(self.outstanding.values()) {
            remove(&copy.at);
        }
    }
}

fn no_arm(missing: Missing) -> AxError {
    AxError::failure(
        AxCode::SandboxDenied,
        "run outside the confinement",
        missing.phrase(),
    )
    .with_recovery(missing.recovery())
}

#[cfg(not(windows))]
fn no_job_object() -> AxError {
    AxError::failure(
        AxCode::SandboxDenied,
        "run under a job object",
        "this build makes no job object",
    )
    .with_recovery(
        "run under the copied tree instead: it copies the working directory and does \
         not isolate the network, so keep the command off the network or run it on a \
         machine that has the arm",
    )
}

pub(super) mod copy;

use copy::{Budget, Stage, fresh, mirror, remove};

fn namespaced(wrapper: &Path, copy: &Path, workdir: &Path, command: &Command) -> Command {
    let mut wrapped = child::command(wrapper);
    wrapped
        .current_dir(workdir)
        .arg("--ro-bind")
        .arg("/")
        .arg("/")
        .arg("--bind")
        .arg(copy)
        .arg(workdir)
        .arg("--chdir")
        .arg(workdir)
        .arg("--proc")
        .arg("/proc")
        .arg("--dev")
        .arg("/dev")
        .arg("--unshare-all");
    #[cfg(target_os = "linux")]
    wrapped.arg("--unshare-user");
    wrapped
        .arg("--die-with-parent")
        .arg("--")
        .arg(command.get_program());
    for arg in command.get_args() {
        wrapped.arg(arg);
    }
    for (name, value) in command.get_envs() {
        match value {
            Some(value) => wrapped.env(name, value),
            None => wrapped.env_remove(name),
        };
    }
    wrapped
}
