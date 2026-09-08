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

use kernel::{
    AxCode, AxError, CostTier, Effect, ExecArm, Payload, RenderIntent, Temporal, Tool, ToolCall,
    ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::sandbox::{Fuel, Mount, Sandbox, SandboxExit, SandboxJob};

/// Environment variables a child may inherit. Everything else is
/// dropped: an allowlist stays safe when the process environment grows,
/// which a denylist does not.
const ENV_ALLOWLIST: [&str; 4] = ["PATH", "LANG", "LC_ALL", "TZ"];

pub struct ExecTool {
    workdir: PathBuf,
    mounts: Vec<Mount>,
    python_wasm: Option<PathBuf>,
    sandbox: Box<dyn Sandbox>,
    shell: Option<PathBuf>,
    fuel: Fuel,
    meta: ToolMeta,
}

impl ExecTool {
    pub fn new(
        workdir: PathBuf,
        mounts: Vec<Mount>,
        python_wasm: Option<PathBuf>,
        sandbox: Box<dyn Sandbox>,
        shell: Option<PathBuf>,
        fuel: Fuel,
        domain: kernel::Address,
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
        Ok(ExecTool {
            workdir,
            mounts,
            python_wasm,
            sandbox,
            shell,
            fuel,
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
        command.current_dir(&self.workdir).args(args).env_clear();
        for key in ENV_ALLOWLIST {
            if let Ok(value) = std::env::var(key) {
                command.env(key, value);
            }
        }
        let output = command.output().map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "run program",
                format!("{path}: {err}"),
            )
            .with_recovery("check the program name, or use the shell arm")
        })?;
        let code = output.status.code().unwrap_or(-1);
        outcome(&output.stdout, &output.stderr, i64::from(code), "program")
    }

    fn run_python(&mut self, code: &str) -> Result<ToolOutcome, AxError> {
        let Some(wasm) = self.python_wasm.clone() else {
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
            mounts: self.mounts.clone(),
            fuel: self.fuel,
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
        outcome(&result.stdout, &result.stderr, exit_code, "python")
    }

    fn run_shell(&self, text: &str) -> Result<ToolOutcome, AxError> {
        let Some(shell) = &self.shell else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "run shell",
                "no shell interpreter was found",
            )
            .with_recovery("use the program arm with an explicit executable"));
        };
        let flag = if cfg!(windows) { "/C" } else { "-c" };
        let mut command = std::process::Command::new(shell);
        command
            .current_dir(&self.workdir)
            .arg(flag)
            .arg(text)
            .env_clear();
        for key in ENV_ALLOWLIST {
            if let Ok(value) = std::env::var(key) {
                command.env(key, value);
            }
        }
        let output = command.output().map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "run shell",
                format!("{}: {err}", shell.display()),
            )
        })?;
        let code = output.status.code().unwrap_or(-1);
        outcome(&output.stdout, &output.stderr, i64::from(code), "shell")
    }
}

fn outcome(
    stdout: &[u8],
    stderr: &[u8],
    exit_code: i64,
    arm: &str,
) -> Result<ToolOutcome, AxError> {
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String(arm.to_owned()));
    result.insert(
        "stdout".to_owned(),
        Value::String(String::from_utf8_lossy(stdout).into_owned()),
    );
    result.insert(
        "stderr".to_owned(),
        Value::String(String::from_utf8_lossy(stderr).into_owned()),
    );
    result.insert("exit_code".to_owned(), Value::Number(exit_code.into()));
    Ok(ToolOutcome {
        result: Payload::new(result)?,
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
        match parse_arm(call.args.as_map())? {
            ExecArm::Program { path, args } => self.run_program(&path, &args),
            ExecArm::Python { code } => self.run_python(&code),
            ExecArm::Shell { text } => self.run_shell(&text),
        }
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
