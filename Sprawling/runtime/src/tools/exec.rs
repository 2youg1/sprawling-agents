// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use kernel::{
    AxCode, AxError, CostTier, Effect, EnvVarName, ExecArm, Payload, RenderIntent, RunId, Temporal,
    Tool, ToolCall, ToolDef, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::backlog::{Backlog, Shares};
use crate::reserve::Reservation;

mod confinement;
pub(crate) mod container;
mod dispatch;
#[cfg(any(target_os = "macos", test))]
mod native_macos;
#[cfg(windows)]
pub(crate) mod native_windows;
mod program_line;
pub use container::{ContainerEngine, ContainerLaunch, ContainerRuntime};
pub(crate) use program_line::program_line;
mod shell;
mod yielding;

pub use confinement::{
    Assurances, Confined, Confinement, Guarantee, Kept, Missing, Offerings, Placed, Placement,
    parse_placement,
};
pub use outcome::{FailureClass, ShellCount, ShellTally};
pub use shell::Shell;

const COMMAND: &str = "command";
const PYTHON: &str = "python";

const ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"];

pub struct ExecSetup {
    pub workdir: PathBuf,
    pub python: Option<PathBuf>,
    pub shell: Shell,

    pub env_passthrough: Vec<EnvVarName>,
    pub domain: kernel::Address,

    pub run: RunId,
    pub policy: crate::mode::PolicyReader,
}

pub struct ExecTool {
    setup: ExecSetup,

    backlog: Backlog,
    confinement: Mutex<Confined>,
    meta: ToolMeta,
    container: Option<(kernel::ContainerLimits, ContainerRuntime)>,
    program_route: dispatch::ProgramRoute,
}

impl ExecTool {
    pub fn definition(disclosure: String) -> Result<ToolDef, AxError> {
        Ok(ToolDef {
            name: ToolName::parse("exec")?,
            description: disclosure,
            input_schema: Payload::of(&serde_json::json!({
                "type": "object",
                "properties": {
                    COMMAND: { "type": "string", "description": "the command to execute" },
                    PYTHON: { "type": "string", "description": "a Python script to run instead \
                        of a command: a short script against the standard library - pathlib, \
                        difflib, re, itertools, collections - with no classes, no exception \
                        handlers and no comments; read the error when it fails" },
                    "where": { "type": "string", "enum": ["sandbox", "host"], "description":
                        "where the command or script runs: sandbox (the default) or host" },
                },
            }))?,
        })
    }

    pub fn standing_definition(shell: bool) -> Result<ToolDef, AxError> {
        let confinement = Confined::detect();
        let shell_sentence = if shell {
            " A command runs under the shell this building configured.".to_owned()
        } else {
            shell::Shell::Absent.statement()
        };
        Self::definition(disclosure::sentence(
            &confinement.statement(),
            &shell_sentence,
        ))
    }

    pub fn new(setup: ExecSetup, backlog: Backlog) -> Result<ExecTool, AxError> {
        let domain = setup.domain.clone();
        let confinement = Confined::detect();
        let defined = Self::definition(disclosure::describe(&setup, &confinement))?;
        Ok(ExecTool {
            setup,
            container: None,
            program_route: dispatch::ProgramRoute::Confined,
            backlog,
            confinement: Mutex::new(confinement),
            meta: ToolMeta {
                name: defined.name,
                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Write { domain },
                cost_tier: CostTier::Heavy,
                timeout: None,
                render: RenderIntent::Terminal,
                temporal: Temporal::Timestamped,
            },
        })
    }

    fn run_program(
        &self,
        path: &str,
        args: &[String],
        placement: Placement,
    ) -> Result<ToolOutcome, AxError> {
        let mut command = child::command(path);
        command.current_dir(&self.setup.workdir).args(args);
        let what = if args.is_empty() {
            path.to_owned()
        } else {
            format!("{path} {}", args.join(" "))
        };
        self.through_the_backlog(command, what, "program", placement)
    }

    fn run_python(&self, code: &str, placement: Placement) -> Result<ToolOutcome, AxError> {
        let Some(python) = self.setup.python.as_ref() else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "run python",
                "no Python interpreter is configured on this machine",
            )
            .with_recovery(
                "install Python 3 on PATH as python3 or python, set SPRAWLING_PYTHON to an \
                 interpreter, or pass the work as a `command`",
            ));
        };
        let mut command = child::command(python);
        command.current_dir(&self.setup.workdir).args(["-c", code]);
        let what = format!("{} -c <script>", display_program(python));
        self.through_the_backlog(command, what, "python", placement)
    }

    fn run_shell(&self, text: &str, placement: Placement) -> Result<ToolOutcome, AxError> {
        let (mut command, interpreter) = match (placement, &self.container) {
            (Placement::Sandbox, Some(_)) => self.setup.shell.container_command(text)?,
            (Placement::Host, _) | (Placement::Sandbox, None) => self.setup.shell.command(text)?,
        };
        command.current_dir(&self.setup.workdir);
        let answer = self.through_the_backlog(command, text.to_owned(), "shell", placement)?;
        with_interpreter(answer, &interpreter)
    }

    fn reservation(&self) -> Option<Reservation> {
        match self.backlog.shares() {
            Shares::CpuAndMemory { limit } => Some(self.backlog.budget().reserve(limit.get())),
            Shares::Unset | Shares::Cpu => None,
        }
    }
}

fn display_program(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

mod disclosure;
mod outcome;

use outcome::{with_backlog, with_interpreter};

pub fn parse_arm(args: &Map<String, Value>) -> Result<ExecArm, AxError> {
    match (text_of(args, COMMAND)?, text_of(args, PYTHON)?) {
        (Some(text), None) => Ok(ExecArm::Shell { text }),
        (None, Some(code)) => Ok(ExecArm::Python { code }),
        (Some(_), Some(_)) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "run exec",
            "both `command` and `python`",
        )
        .with_recovery("pass one of them; a call runs one thing")),
        (None, None) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "run exec",
            if args.contains_key("arm") {
                "`arm` is how an older build spelled this call"
            } else {
                "neither `command` nor `python`"
            },
        )
        .with_recovery("pass `command` with the command line to run, or `python` with a script")),
    }
}

fn text_of(args: &Map<String, Value>, key: &str) -> Result<Option<String>, AxError> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(Some(text.clone())),
        Some(_) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "run exec",
            format!("`{key}` is not a string with something in it"),
        )
        .with_recovery(format!("pass `{key}` as the text to run"))),
    }
}

impl Tool for ExecTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "run exec",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        crate::mode::mode_admits(&self.setup.policy.now(), kernel::ToolName::EXEC)?;
        let placement = parse_placement(call.args.as_map())?.opened_by(&self.setup)?;
        let answer = match parse_arm(call.args.as_map())? {
            ExecArm::Shell { text } if self.setup.shell == Shell::Absent => {
                let (path, args) = program_line::program_line(&text)?;
                self.run_program(&path, &args, placement)
            }
            ExecArm::Program { path, args } => self.run_program(&path, &args, placement),

            ExecArm::Python { code } => self.run_python(&code, placement),
            ExecArm::Shell { text } => self.run_shell(&text, placement),
        }?;
        let finished = self.backlog.harvest(self.setup.run)?;
        held(&self.confinement)?.reaped(&finished);
        with_backlog(answer, finished)
    }
}

fn held<T>(lock: &Mutex<T>) -> Result<MutexGuard<'_, T>, AxError> {
    lock.lock().map_err(|_| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "run exec",
            "a previous command left this tool locked when its thread died",
        )
        .with_recovery("dispatch the work again; the next run starts with a fresh exec tool")
    })
}

impl Drop for ExecTool {
    fn drop(&mut self) {
        self.backlog.release(self.setup.run);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
