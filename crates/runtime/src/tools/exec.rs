// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The exec tool: exactly three arms, each with its own failure story.
//!
//! A call names a `command` or a `python` script, the way Claude Code's
//! `Bash` takes a `command` (`crates/runtime/spec/Tools/Exec.lean`
//! §8-13-4, D118). The building decides which arm a command takes: the
//! shell when it offers one, and otherwise the program the line names,
//! read by `program_line`, which refuses shell syntax rather than pass
//! it on as text.
//!
//! Program runs a host process with a pinned working directory and an
//! environment allowlist — secrets are never passed through, because a
//! child process inherits whatever it is given and cannot be asked to
//! forget. Its chosen confinement applies unless the call names `where: host`; disclosure states
//! which arm that is and what the arm does not hold. Python runs inside
//! the wasip1 sandbox, where the capability surface is the mount list and
//! there is no network at all. A building that asked for a shell this
//! machine does not have is refused, rather than having its line run
//! as a program: a line written for a shell is a different command
//! without one.
//!
//! A missing component is `E_TOOL_UNAVAILABLE` carrying the alternative
//! that would work, so the caller redirects instead of guessing.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use kernel::{
    AxCode, AxError, CostTier, Effect, EnvVarName, ExecArm, Payload, RenderIntent, RunId, Temporal,
    Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::backlog::{Backlog, Exit, Shares};
use crate::reserve::Reservation;
use crate::sandbox::{Fuel, GuestMemory, Mount, Sandbox, SandboxExit, SandboxJob, SandboxOutcome};

mod confinement;
pub(crate) mod container;
mod dispatch;
#[cfg(any(target_os = "macos", test))]
mod native_macos;
#[cfg(windows)]
pub(crate) mod native_windows;
mod program_line;
pub use container::{ContainerEngine, ContainerLaunch, ContainerRuntime};
mod shell;
mod yielding;

pub use confinement::{
    Assurances, Confined, Confinement, Guarantee, Kept, Missing, Offerings, Placed, Placement,
    parse_placement,
};
pub use outcome::{FailureClass, ShellCount, ShellTally};
pub use shell::Shell;

/// The argument names a call is read by, spelled once for the schema,
/// the reading and the refusals.
const COMMAND: &str = "command";
const PYTHON: &str = "python";

/// Environment variables a child may inherit. Everything else is
/// dropped: an allowlist stays safe when the process environment grows,
/// which a denylist does not.
const ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"];

/// What one run's execution boundary is made of.
///
/// Values that always travel together and are never chosen apart: read
/// out of one frozen configuration, one machine and one dispatch, they
/// reach this tool as one thing rather than as a parameter list nobody
/// can call correctly from memory. `policy` is the run's cell (§8-55, §8-62).
pub struct ExecSetup {
    pub workdir: PathBuf,
    pub mounts: Vec<Mount>,
    pub python_wasm: Option<PathBuf>,
    pub shell: Shell,
    pub fuel: Fuel,
    /// The names this building declared its children may inherit, on top
    /// of [`ENV_ALLOWLIST`].
    pub env_passthrough: Vec<EnvVarName>,
    pub domain: kernel::Address,
    /// The run this tool serves: a command it hands to the background
    /// is owed to this run, and its output reaches no other.
    pub run: RunId,
    pub policy: crate::mode::PolicyReader,
}

pub struct ExecTool {
    setup: ExecSetup,
    /// `Sandbox::run` mutates the guest; `invoke` borrows the tool.
    sandbox: Mutex<Box<dyn Sandbox>>,
    backlog: Backlog,
    confinement: Mutex<Confined>,
    meta: ToolMeta,
    container: Option<(kernel::ContainerLimits, ContainerRuntime)>,
    program_route: dispatch::ProgramRoute,
}

impl ExecTool {
    pub fn new(
        setup: ExecSetup,
        sandbox: Box<dyn Sandbox>,
        backlog: Backlog,
    ) -> Result<ExecTool, AxError> {
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                COMMAND: { "type": "string", "description": "the command to execute" },
                PYTHON: { "type": "string", "description": "a Python script to run instead \
                    of a command: a short script against the standard library - pathlib, \
                    difflib, re, itertools, collections - with no classes, no exception \
                    handlers and no comments; read the error when it fails" },
                "where": { "type": "string", "enum": ["sandbox", "host"], "description":
                    "where the command runs: sandbox (the default) or host" },
            },
        });
        let domain = setup.domain.clone();
        let confinement = Confined::detect();
        let disclosure = disclosure::describe(&setup, &confinement);
        Ok(ExecTool {
            setup,
            container: None,
            program_route: dispatch::ProgramRoute::Confined,
            sandbox: Mutex::new(sandbox),
            backlog,
            confinement: Mutex::new(confinement),
            meta: ToolMeta {
                name: ToolName::parse("exec")?,
                disclosure,
                params: Payload::of(&params)?,
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

    fn run_python(&self, code: &str) -> Result<ToolOutcome, AxError> {
        let Some(wasm) = self.setup.python_wasm.clone() else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "run python",
                "no CPython-WASI component is configured",
            )
            .with_recovery(
                "pass the work as a `command`, or ask the User to configure a CPython-WASI \
                 component",
            ));
        };
        let job = SandboxJob {
            wasm,
            argv: vec!["python".to_owned(), "-c".to_owned(), code.to_owned()],
            env: Vec::new(),
            stdin: Vec::new(),
            mounts: self.setup.mounts.clone(),
            fuel: self.setup.fuel,
            memory: GuestMemory::of_shares(self.backlog.shares()),
        };
        let reservation = self.reservation();
        let result = held(&self.sandbox)?.run(&job)?;
        let waited = reservation.as_ref().and_then(Reservation::waited);
        drop(reservation);
        // A guest refused at its run's ceiling says so under the same key
        // a command does, whichever way it then ended (`Sandbox.lean` D102).
        with_wait(
            with_ceiling(python_answer(&result)?, result.ceiling)?,
            waited,
        )
    }

    /// One heavy call's reservation from the machine budget: the run's
    /// ceiling when the run asks for one, and none otherwise
    /// (`crates/runtime/spec/Reserve.lean` D104). Blocks until the line
    /// starts it, holding no other lock of this tool while it waits.
    fn reservation(&self) -> Option<Reservation> {
        match self.backlog.shares() {
            Shares::CpuAndMemory { limit } => Some(self.backlog.budget().reserve(limit.get())),
            Shares::Unset | Shares::Cpu => None,
        }
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
}

/// The python arm's result for how its guest ended.
fn python_answer(result: &SandboxOutcome) -> Result<ToolOutcome, AxError> {
    let exit = match &result.exit {
        SandboxExit::Success => Exit::Ended { code: 0 },
        SandboxExit::Failure { code } => Exit::Ended {
            code: i32::try_from(*code).unwrap_or(i32::MAX),
        },
        // Exhaustion and traps are guest facts the caller must see as
        // themselves, not flattened into a generic non-zero exit.
        SandboxExit::FuelExhausted => {
            return exceptional(&result.stdout, &result.stderr, "fuel_exhausted", None);
        }
        SandboxExit::Trap { message } => {
            return exceptional(
                &result.stdout,
                &result.stderr,
                "trap",
                Some(message.clone()),
            );
        }
    };
    settled(
        &String::from_utf8_lossy(&result.stdout),
        &String::from_utf8_lossy(&result.stderr),
        exit,
        "python",
    )
}

mod disclosure;
mod outcome;

use outcome::{exceptional, settled, with_backlog, with_ceiling, with_interpreter, with_wait};

fn no_host_python() -> AxError {
    AxError::failure(
        AxCode::SandboxDenied,
        "run python on the host",
        "the python arm runs a wasip1 guest and has no host form",
    )
    .with_recovery(
        "pass a `command` that runs a host interpreter, or leave `where` out to run the \
         configured component",
    )
}

/// Reads what the call asks to run: a `command` is a shell line here,
/// whichever arm the building later gives it, so the deletion forecast
/// and the sieve read the same text in every building; a `python` is a
/// script (§8-13-4). A call that names neither, or both, is refused
/// rather than guessed at, and so is the older `arm` spelling (D115).
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

/// One of the two texts a call can carry: absent, or a string that says
/// something.
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
            // A building that offers no shell runs the program the line
            // names (D118).
            ExecArm::Shell { text } if self.setup.shell == Shell::Absent => {
                let (path, args) = program_line::program_line(&text)?;
                self.run_program(&path, &args, placement)
            }
            ExecArm::Program { path, args } => self.run_program(&path, &args, placement),
            // The Python arm has no host form: its interpreter is a
            // component this build runs as a wasip1 guest, and a host
            // `python` would be a different program than the one the
            // arm promises.
            ExecArm::Python { code } => match placement {
                Placement::Sandbox => self.run_python(&code),
                Placement::Host => Err(no_host_python()),
            },
            ExecArm::Shell { text } => self.run_shell(&text, placement),
        }?;
        let finished = self.backlog.harvest(self.setup.run)?;
        held(&self.confinement)?.reaped(&finished);
        with_backlog(answer, finished)
    }
}

/// One of this tool's own locks, or the refusal that says a thread died
/// holding it.
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

/// Releases background commands when the run drops its tool bench.
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
