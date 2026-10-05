// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Frozen program placement (`crates/runtime/spec/Tools/Exec/Container.lean`).

use std::collections::BTreeMap;
use std::sync::Mutex;

use super::outcome::{backgrounded, settled, with_environment};
use super::{
    Confined, Confinement, ContainerRuntime, ENV_ALLOWLIST, ExecTool, Placement, Shell, held,
    yielding,
};
use crate::backlog::Started;
use kernel::{AxCode, AxError, EnvVarName, ToolOutcome};

#[derive(Clone, Copy)]
pub(super) enum ProgramRoute {
    Confined,
    Host,
    Python,
}

impl ExecTool {
    /// Uses an explicitly admitted daemon and frozen limits for program and shell calls.
    /// The daemon must be discovered by the host, rather than by a second configuration path.
    #[must_use]
    pub fn with_container(
        mut self,
        limits: kernel::ContainerLimits,
        runtime: ContainerRuntime,
    ) -> Self {
        self.program_route = ProgramRoute::Confined;
        self.container = Some((limits, runtime));
        self.meta.disclosure = format!(
            "Run a program, Python snippet, or shell line. Program and shell sandbox calls use              an explicitly configured Linux container: writes land on a copy, network is closed,              a non-root user and CPU/memory/process limits are inspected before start.              Daemon cleanup failures are reported and retained for retry. `where: host` asks              for host execution.{}",
            match self.setup.shell {
                Shell::Absent => "",
                Shell::Missing {
                    asked: kernel::Interpreter::Pwsh,
                }
                | Shell::Found {
                    interpreter: kernel::Interpreter::Pwsh,
                    ..
                } => " Container shell lines use pwsh.",
                Shell::Missing {
                    asked: kernel::Interpreter::System,
                }
                | Shell::Found {
                    interpreter: kernel::Interpreter::System,
                    ..
                } => " Container shell lines use /bin/sh.",
            }
        );
        self
    }

    /// Chooses an execution boundary before the tool is admitted to a run.
    /// The disclosure and actual placement always read the same confinement.
    pub fn confined(mut self, confinement: Confined) -> Self {
        self.program_route = ProgramRoute::Confined;
        self.container = None;
        self.meta.disclosure = super::disclosure::describe(&self.setup, &confinement);
        self.confinement = Mutex::new(confinement);
        self
    }

    /// Places sandbox program calls on the explicitly selected host arm.
    #[must_use]
    pub fn on_host(mut self) -> Self {
        self.container = None;
        self.program_route = ProgramRoute::Host;
        self.meta.disclosure = format!(
            "Program and shell calls run on the host with no confinement.{}",
            self.setup.shell.statement()
        );
        self
    }

    /// Offers the existing WASI guest while refusing sandbox program and shell calls.
    #[must_use]
    pub fn python_only(mut self) -> Self {
        self.container = None;
        self.program_route = ProgramRoute::Python;
        self.meta.disclosure = "Sandbox execution offers only the Python WASI arm; program and shell sandbox calls refuse.".to_owned();
        self
    }

    /// Every host command goes through the table, whichever arm asked
    /// for it: there is no `background` argument, because two paths would
    /// be two authorities and the one with the hole in it would always be
    /// the one nobody remembered. The command is lowered before its
    /// environment is cleared, so the clearing lands on whatever process
    /// is actually spawned.
    pub(super) fn through_the_backlog(
        &self,
        command: std::process::Command,
        what: String,
        arm: &str,
        placement: Placement,
    ) -> Result<ToolOutcome, AxError> {
        let placement = match (placement, self.program_route) {
            (Placement::Sandbox, ProgramRoute::Host) => Placement::Host.opened_by(&self.setup)?,
            (Placement::Sandbox, ProgramRoute::Python) => {
                return Err(AxError::failure(
                    AxCode::SandboxDenied,
                    "execute a program",
                    "the frozen sandbox arm only offers Python WASI",
                )
                .with_recovery(
                    "use the Python arm, or select a program-capable sandbox for the next run",
                ));
            }
            (Placement::Sandbox, ProgramRoute::Confined) | (Placement::Host, _) => placement,
        };
        let mut command = match (placement, &self.container) {
            (Placement::Sandbox, _) => command,
            (Placement::Host, _) => yielding::one_level_down(command, self.backlog.shares())?,
        };
        let inherited = self.inherited_environment();
        command.env_clear();
        for (key, value) in &inherited {
            command.env(key, value);
        }
        let (started, placed) = match (placement, &self.container) {
            (Placement::Sandbox, Some((limits, runtime))) => {
                let mut copy =
                    Confined::with_arm(Confinement::CopiedTree, Some(std::env::temp_dir()));
                let (target, placed) = copy.place(command, &self.setup.workdir)?;
                let copy = placed.into_container_copy()?;
                (
                    self.backlog.run_container(
                        self.setup.run,
                        &self.setup.domain,
                        what,
                        crate::backlog::ContainerRequest {
                            runtime: runtime.clone(),
                            limits: limits.clone(),
                            copy,
                            target,
                        },
                    )?,
                    None,
                )
            }
            (Placement::Sandbox, None) => {
                let (mut command, placed) = held(&self.confinement)?.prepare(
                    command,
                    &self.setup.workdir,
                    self.backlog.shares(),
                )?;
                command.env_clear();
                for (key, value) in &inherited {
                    command.env(key, value);
                }
                #[cfg(windows)]
                let started = if matches!(
                    held(&self.confinement)?.arm(),
                    Confinement::WindowsJobObject
                ) {
                    self.backlog
                        .run_native(self.setup.run, &self.setup.domain, what, command)?
                } else {
                    self.backlog
                        .run(self.setup.run, &self.setup.domain, what, command)?
                };
                #[cfg(not(windows))]
                let started =
                    self.backlog
                        .run(self.setup.run, &self.setup.domain, what, command)?;
                (started, Some(placed))
            }
            (Placement::Host, _) => (
                self.backlog
                    .run(self.setup.run, &self.setup.domain, what, command)?,
                None,
            ),
        };
        let result = match started {
            Started::Settled {
                exit,
                stdout,
                stderr,
            } => {
                if let Some(placed) = placed {
                    held(&self.confinement)?.settled(placed);
                }
                settled(&stdout, &stderr, exit, arm)?
            }
            Started::Backgrounded { id, what } => {
                if let Some(placed) = placed {
                    held(&self.confinement)?.handed(id, placed);
                }
                backgrounded(&id, &what, arm)?
            }
        };
        with_environment(result, &inherited)
    }

    /// The variables this run's children actually get: the floor every
    /// city grants, plus the names this building declared, minus every
    /// name this machine does not set.
    ///
    /// A name with no value on this machine is left out rather than set
    /// empty, because an empty variable and an absent one are two
    /// different things to the programs that read them.
    fn inherited_environment(&self) -> BTreeMap<String, String> {
        let mut chosen = BTreeMap::new();
        let declared = self
            .setup
            .env_passthrough
            .iter()
            .map(EnvVarName::as_str)
            .chain(ENV_ALLOWLIST);
        for key in declared {
            if let Ok(value) = std::env::var(key) {
                chosen.insert(key.to_owned(), value);
            }
        }
        chosen
    }
}
