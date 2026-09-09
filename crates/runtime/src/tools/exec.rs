// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The exec tool: exactly three arms, each with its own failure story.
//!
//! Program runs a host process with a pinned working directory and an
//! environment allowlist — secrets are never passed through, because a
//! child process inherits whatever it is given and cannot be asked to
//! forget. Python runs inside the sandbox, where the capability surface
//! is the mount list and there is no network at all. Shell probes for
//! an interpreter and **refuses when there is none**, rather than
//! quietly rewriting the request as a Program call: a shell line that
//! silently becomes something else is a worse answer than a refusal
//! naming what is missing.
//!
//! A missing component is `E_TOOL_UNAVAILABLE` carrying the alternative
//! that would work, so the caller redirects instead of guessing.

use std::path::PathBuf;

use std::collections::BTreeMap;

use kernel::{
    AxCode, AxError, CostTier, Effect, EnvVarName, ExecArm, Payload, RenderIntent, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::backlog::{Backlog, BacklogId, Finished, Started};
use crate::sandbox::{Fuel, Mount, Sandbox, SandboxExit, SandboxJob};

/// Environment variables a child may inherit. Everything else is
/// dropped: an allowlist stays safe when the process environment grows,
/// which a denylist does not.
const ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"];

/// What one building's execution boundary is made of.
///
/// Seven values that always travel together and are never chosen
/// independently: they are read out of one frozen configuration and one
/// machine, and they reach this tool as one thing rather than as a
/// parameter list nobody can call correctly from memory.
pub struct ExecSetup {
    pub workdir: PathBuf,
    pub mounts: Vec<Mount>,
    pub python_wasm: Option<PathBuf>,
    pub shell: Option<PathBuf>,
    pub fuel: Fuel,
    /// The names this building declared its children may inherit, on top
    /// of [`ENV_ALLOWLIST`].
    pub env_passthrough: Vec<EnvVarName>,
    pub domain: kernel::Address,
}

pub struct ExecTool {
    setup: ExecSetup,
    sandbox: Box<dyn Sandbox>,
    backlog: Backlog,
    meta: ToolMeta,
}

impl ExecTool {
    pub fn new(
        setup: ExecSetup,
        sandbox: Box<dyn Sandbox>,
        backlog: Backlog,
    ) -> Result<ExecTool, AxError> {
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        let mut properties = Map::new();
        let mut arm = Map::new();
        arm.insert("type".to_owned(), Value::String("object".to_owned()));
        arm.insert(
            "description".to_owned(),
            Value::String(
                "one of {program:{path,args}}, {python:{code}}, {shell:{text}}".to_owned(),
            ),
        );
        properties.insert("arm".to_owned(), Value::Object(arm));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("arm".to_owned())]),
        );
        let domain = setup.domain.clone();
        Ok(ExecTool {
            setup,
            sandbox,
            backlog,
            meta: ToolMeta {
                name: ToolName::parse("exec")?,
                disclosure: "Run a program, a Python snippet, or a shell line.".to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Write { domain },
                cost_tier: CostTier::Heavy,
                timeout: None,
                render: RenderIntent::Terminal,
                temporal: Temporal::Timestamped,
            },
        })
    }

    fn run_program(&self, path: &str, args: &[String]) -> Result<ToolOutcome, AxError> {
        let mut command = std::process::Command::new(path);
        command.current_dir(&self.setup.workdir).args(args);
        let what = if args.is_empty() {
            path.to_owned()
        } else {
            format!("{path} {}", args.join(" "))
        };
        self.through_the_backlog(command, what, "program")
    }

    /// Every host command goes through the table, whichever arm asked
    /// for it.
    ///
    /// There is no `background` argument, because two paths would be two
    /// authorities and the one with the hole in it would always be the
    /// one nobody remembered.
    fn through_the_backlog(
        &self,
        mut command: std::process::Command,
        what: String,
        arm: &str,
    ) -> Result<ToolOutcome, AxError> {
        let inherited = self.inherited_environment();
        command.env_clear();
        for (key, value) in &inherited {
            command.env(key, value);
        }
        let started = self.backlog.run(&self.setup.domain, what, command)?;
        let result = match started {
            Started::Settled {
                exit_code,
                stdout,
                stderr,
            } => outcome(&stdout, &stderr, exit_code, arm)?,
            Started::Backgrounded { id, what } => backgrounded(&id, &what, arm)?,
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

    fn run_python(&mut self, code: &str) -> Result<ToolOutcome, AxError> {
        let Some(wasm) = self.setup.python_wasm.clone() else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "run python",
                "no CPython-WASI component is configured",
            )
            .with_recovery("use the program arm, or configure a CPython-WASI component"));
        };
        let job = SandboxJob {
            wasm,
            argv: vec!["python".to_owned(), "-c".to_owned(), code.to_owned()],
            env: Vec::new(),
            stdin: Vec::new(),
            mounts: self.setup.mounts.clone(),
            fuel: self.setup.fuel,
        };
        let result = self.sandbox.run(&job)?;
        let exit_code = match &result.exit {
            SandboxExit::Success => 0,
            SandboxExit::Failure { code } => i64::try_from(*code).unwrap_or(i64::MAX),
            // Exhaustion and traps are guest facts the caller must see
            // as themselves, not flattened into a generic non-zero exit.
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
        outcome(
            &String::from_utf8_lossy(&result.stdout),
            &String::from_utf8_lossy(&result.stderr),
            exit_code,
            "python",
        )
    }

    fn run_shell(&self, text: &str) -> Result<ToolOutcome, AxError> {
        let Some(shell) = &self.setup.shell else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "run shell",
                "no shell interpreter was found",
            )
            .with_recovery("use the program arm with an explicit executable"));
        };
        let flag = if cfg!(windows) { "/C" } else { "-c" };
        let mut command = std::process::Command::new(shell);
        command.current_dir(&self.setup.workdir).arg(flag).arg(text);
        self.through_the_backlog(command, text.to_owned(), "shell")
    }
}

/// What a caller is told about a command that outlived its window.
///
/// The handle and the sentence travel together: an agent that is given
/// an identifier and no instruction waits for it anyway, which is the
/// behaviour this whole table exists to stop.
fn backgrounded(id: &BacklogId, what: &str, arm: &str) -> Result<ToolOutcome, AxError> {
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String(arm.to_owned()));
    result.insert(
        "outcome".to_owned(),
        Value::String("backgrounded".to_owned()),
    );
    result.insert("handle".to_owned(), Value::String(id.to_string()));
    result.insert("what".to_owned(), Value::String(what.to_owned()));
    result.insert(
        "detail".to_owned(),
        Value::String(
            "still running; do not wait for it - carry on, and its result arrives at the end \
             of a later tool result"
                .to_owned(),
        ),
    );
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

/// Adds every background member that has stopped since the last call to
/// the tail of this result.
///
/// It is the tail rather than the head because the answer the caller
/// asked for is the one it is reading for; what arrived while it was
/// working comes after.
fn with_backlog(outcome: ToolOutcome, done: Vec<Finished>) -> Result<ToolOutcome, AxError> {
    if done.is_empty() {
        return Ok(outcome);
    }
    let mut result = outcome.result.as_map().clone();
    let rows = done
        .into_iter()
        .map(|member| {
            let mut row = Map::new();
            row.insert("handle".to_owned(), Value::String(member.id.to_string()));
            row.insert("what".to_owned(), Value::String(member.what));
            row.insert(
                "exit_code".to_owned(),
                Value::Number(member.exit_code.into()),
            );
            row.insert("stdout".to_owned(), Value::String(member.stdout));
            row.insert("stderr".to_owned(), Value::String(member.stderr));
            Value::Object(row)
        })
        .collect();
    result.insert("background".to_owned(), Value::Array(rows));
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

fn outcome(stdout: &str, stderr: &str, exit_code: i64, arm: &str) -> Result<ToolOutcome, AxError> {
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String(arm.to_owned()));
    result.insert("stdout".to_owned(), Value::String(stdout.to_owned()));
    result.insert("stderr".to_owned(), Value::String(stderr.to_owned()));
    result.insert("exit_code".to_owned(), Value::Number(exit_code.into()));
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

/// Adds the names this call's child inherited to its result.
///
/// Names only, never values: which names a run inherited is a fact the
/// ledger keeps, and what those names held is a fact it must not.
fn with_environment(
    outcome: ToolOutcome,
    inherited: &BTreeMap<String, String>,
) -> Result<ToolOutcome, AxError> {
    let mut result = outcome.result.as_map().clone();
    result.insert(
        "env".to_owned(),
        Value::Array(
            inherited
                .keys()
                .map(|name| Value::String(name.clone()))
                .collect(),
        ),
    );
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

fn exceptional(
    stdout: &[u8],
    stderr: &[u8],
    kind: &str,
    detail: Option<String>,
) -> Result<ToolOutcome, AxError> {
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String("python".to_owned()));
    result.insert(
        "stdout".to_owned(),
        Value::String(String::from_utf8_lossy(stdout).into_owned()),
    );
    result.insert(
        "stderr".to_owned(),
        Value::String(String::from_utf8_lossy(stderr).into_owned()),
    );
    result.insert("outcome".to_owned(), Value::String(kind.to_owned()));
    if let Some(detail) = detail {
        result.insert("detail".to_owned(), Value::String(detail));
    }
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: Vec::new(),
    })
}

/// Reads the arm out of the call. An unrecognised shape is refused
/// rather than defaulted to shell — guessing which arm was meant is how
/// a program run becomes a shell injection.
pub fn parse_arm(args: &Map<String, Value>) -> Result<ExecArm, AxError> {
    let arm = args
        .get("arm")
        .ok_or_else(|| AxError::failure(AxCode::InvalidArgs, "run exec", "missing `arm`"))?;
    serde_json::from_value(arm.clone()).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "run exec",
            format!("unrecognised arm: {err}"),
        )
        .with_recovery("use one of program, python, shell")
    })
}

impl Tool for ExecTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "run exec",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            ));
        }
        let answer = match parse_arm(call.args.as_map())? {
            ExecArm::Program { path, args } => self.run_program(&path, &args),
            ExecArm::Python { code } => self.run_python(&code),
            ExecArm::Shell { text } => self.run_shell(&text),
        }?;
        with_backlog(answer, self.backlog.harvest()?)
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
