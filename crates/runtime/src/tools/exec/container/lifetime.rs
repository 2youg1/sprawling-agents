// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Named daemon ownership (`crates/runtime/spec/Tools/Exec/Container.lean`, D50).

use std::path::PathBuf;
use std::process::{Child, Command};

use kernel::{AxError, ContainerLimits};

use crate::backlog::{Exit, Unseen};

use super::cleanup::Cleanup;
use super::guardian::Guard;
use super::{ContainerLaunch, ContainerRuntime, control, denied, inspection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Registered,
    Running,
    Stopping,
    Removed,
}

pub(crate) struct ContainerLease {
    runtime: ContainerRuntime,
    cleanup: Cleanup,
    guardian: Option<Guard>,
    limits: ContainerLimits,
    state: State,
    attach: Option<Child>,
    target_exit: Option<Exit>,
}

impl ContainerLease {
    pub(crate) fn registered(
        runtime: ContainerRuntime,
        name: String,
        copy: PathBuf,
        limits: ContainerLimits,
    ) -> Self {
        Self {
            cleanup: Cleanup::registered(&runtime, name, copy),
            runtime,
            guardian: None,
            limits,
            state: State::Registered,
            attach: None,
            target_exit: None,
        }
    }

    pub(crate) fn start(
        &mut self,
        target: &Command,
        out: std::fs::File,
        err: std::fs::File,
        ready: &std::path::Path,
    ) -> Result<(), AxError> {
        if self.state != State::Registered {
            return Err(denied(
                "start the container",
                "this member already started or was stopped",
            ));
        }
        if let Some(harness) = &self.runtime.guardian {
            self.guardian = Some(Guard::spawn(
                harness,
                &self.cleanup,
                ready,
                err.try_clone()
                    .map_err(|err| denied("share guardian failure output", err))?,
            )?);
        }
        let mut image = self.runtime.command();
        image.args(["image", "inspect", self.limits.image.as_str()]);
        inspection::image(&control::checked(&mut image)?, &self.limits)?;
        let mut create = self.runtime.create_command(
            &self.limits,
            &ContainerLaunch {
                name: &self.cleanup.name,
                copy: &self.cleanup.copy,
                command: target,
            },
        )?;
        control::checked(&mut create)?;
        let mut inspect = self.inspect_command();
        inspection::stopped(
            &control::checked(&mut inspect)?,
            &self.limits,
            self.runtime.engine,
            &self.cleanup.copy,
        )?;
        let mut attach = self.runtime.command();
        attach
            .args(["start", "--attach", &self.cleanup.name])
            .stdin(std::process::Stdio::null())
            .stdout(out)
            .stderr(err);
        self.attach = Some(
            attach
                .spawn()
                .map_err(|err| denied("attach the container", err.to_string()))?,
        );
        self.state = State::Running;
        Ok(())
    }

    pub(crate) fn poll(&mut self) -> Result<Option<Exit>, AxError> {
        match self.state {
            State::Registered => Ok(None),
            State::Removed => Ok(Some(self.target_exit.unwrap_or(Exit::Signalled))),
            State::Stopping => {
                self.remove()?;
                Ok(Some(self.target_exit.unwrap_or(Exit::Signalled)))
            }
            State::Running => {
                let cli = self
                    .attach
                    .as_mut()
                    .ok_or_else(|| denied("poll the container", "missing attach client"))?
                    .try_wait()
                    .map_err(|err| denied("poll the container client", err.to_string()))?;
                if cli.is_none() {
                    return Ok(None);
                }
                let result = control::checked(&mut self.inspect_command())
                    .and_then(|bytes| inspection::state(&bytes));
                match result {
                    Ok((false, code)) => {
                        self.target_exit = Some(Exit::Ended { code });
                        self.remove()?;
                        Ok(Some(Exit::Ended { code }))
                    }
                    Ok((true, _)) => {
                        self.state = State::Stopping;
                        self.remove()?;
                        Err(denied(
                            "wait for the container",
                            "the attach client ended before the target process",
                        ))
                    }
                    Err(err) => {
                        self.target_exit = Some(Exit::Unknown {
                            why: Unseen::WaitRefused,
                        });
                        self.state = State::Stopping;
                        // The result is unread; removal must complete before the member can leave.
                        self.remove()?;
                        Err(err)
                    }
                }
            }
        }
    }

    pub(crate) fn stop(&mut self) -> Result<(), AxError> {
        if self.state != State::Removed {
            self.state = State::Stopping;
            self.remove()?;
        }
        Ok(())
    }

    pub(crate) fn client(&self) -> Option<&Child> {
        self.attach.as_ref()
    }

    fn inspect_command(&self) -> Command {
        let mut command = self.runtime.command();
        command.args(["container", "inspect", &self.cleanup.name]);
        command
    }

    fn remove(&mut self) -> Result<(), AxError> {
        self.state = State::Stopping;
        self.cleanup.remove()?;
        if let Some(child) = &mut self.attach {
            match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => {
                    child
                        .kill()
                        .map_err(|err| denied("stop the attach client", err.to_string()))?;
                    child
                        .wait()
                        .map_err(|err| denied("reap the attach client", err.to_string()))?;
                }
                Err(err) => return Err(denied("reap the attach client", err.to_string())),
            }
        }
        self.cleanup.release_copy()?;
        self.state = State::Removed;
        if let Some(guardian) = &mut self.guardian {
            guardian.disarm()?;
        }
        Ok(())
    }
}

impl Drop for ContainerLease {
    fn drop(&mut self) {
        if self.state != State::Removed
            && let Err(err) = self.stop()
        {
            // No caller survives the city's final drop; preserve the recovery identity on stderr.
            eprintln!(
                "{err}; container {} retains its copy {}",
                self.cleanup.name,
                self.cleanup.copy.display()
            );
        }
    }
}
