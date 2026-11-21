// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Parallel execution with prerequisite propagation. Storage and process
//! failures never prevent unrelated nodes from reporting (xtask D47).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read as _, Seek as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;

use crate::report::XtaskError;

use super::content::Environment;
use super::diagnostic::Diagnostic;
use super::graph::{Operation, Reuse};
use super::store::{Store, Verdict};
use super::suite::PreparedAction;
use super::view::Snapshot;

/// The execution environment shared by every action in one run.
pub(crate) struct Execution<'a> {
    pub(crate) view: &'a Snapshot,
    pub(crate) work: &'a Path,
    pub(crate) store: &'a Store,
    pub(crate) environment: &'a Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Green,
    Refused,
    Broken,
    Blocked,
}

impl Outcome {
    fn verdict(self) -> Option<Verdict> {
        match self {
            Self::Green => Some(Verdict::Green),
            Self::Refused => Some(Verdict::Refused),
            Self::Broken => Some(Verdict::Broken),
            Self::Blocked => None,
        }
    }
}

/// Run the selected graph and return the same exit scale as the gate runner.
pub(crate) fn run(
    execution: &Execution,
    prepared: &[PreparedAction],
    selected: &BTreeSet<String>,
) -> ExitCode {
    let outcomes = schedule(prepared, selected, |action| execute_one(execution, action));
    report(&outcomes)
}

/// Dynamic ready-set scheduler; a failed prerequisite blocks only its dependants.
fn schedule(
    prepared: &[PreparedAction],
    selected: &BTreeSet<String>,
    runner: impl Fn(&PreparedAction) -> Outcome + Sync,
) -> BTreeMap<String, Outcome> {
    let limit = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);
    let mut outcomes = BTreeMap::new();
    let mut running = BTreeSet::new();
    std::thread::scope(|scope| {
        let (send, receive) = mpsc::channel();
        while outcomes.len() < selected.len() {
            let settled_before = outcomes.len();
            for action in prepared
                .iter()
                .filter(|action| selected.contains(action.action.id))
            {
                let id = action.action.id;
                if outcomes.contains_key(id) || running.contains(id) {
                    continue;
                }
                let dependencies: Vec<Outcome> = action
                    .action
                    .needs
                    .iter()
                    .filter_map(|need| outcomes.get(*need).copied())
                    .collect();
                if dependencies.len() != action.action.needs.len() {
                    continue;
                }
                if dependencies
                    .iter()
                    .any(|outcome| *outcome != Outcome::Green)
                {
                    outcomes.insert(id.to_owned(), Outcome::Blocked);
                    println!("blocked {id}: a prerequisite did not pass");
                    continue;
                }
                if running.len() >= limit {
                    continue;
                }
                running.insert(id.to_owned());
                let report = send.clone();
                let execute = &runner;
                scope.spawn(move || {
                    let outcome =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(action)))
                            .unwrap_or(Outcome::Broken);
                    if report.send((id.to_owned(), outcome)).is_err() {
                        eprintln!("ci scheduler: the result receiver closed for {id}");
                    }
                });
            }
            if running.is_empty() {
                if outcomes.len() > settled_before {
                    continue;
                }
                if outcomes.len() < selected.len() {
                    for action in prepared
                        .iter()
                        .filter(|action| selected.contains(action.action.id))
                    {
                        outcomes
                            .entry(action.action.id.to_owned())
                            .or_insert(Outcome::Broken);
                    }
                    eprintln!("ci scheduler: no node can advance; graph validation did not hold");
                }
                break;
            }
            match receive.recv() {
                Ok((id, outcome)) => {
                    running.remove(&id);
                    outcomes.insert(id, outcome);
                }
                Err(err) => {
                    eprintln!("ci scheduler: result channel failed: {err}");
                    break;
                }
            }
        }
    });
    outcomes
}

#[expect(
    clippy::disallowed_methods,
    reason = "developer-tool action timing; never a verification decision"
)]
fn execute_one(execution: &Execution, prepared: &PreparedAction) -> Outcome {
    let store = execution.store;
    let environment = execution.environment;
    let action = prepared.action;
    let policy = environment.reuse(action.reuse);
    let started = std::time::Instant::now();
    let cached = match store.restore(action.id, &prepared.key, environment, policy) {
        Ok(cached) => cached,
        Err(err) => {
            cache_warning(execution.work, action.id, "restore", &err);
            None
        }
    };
    if cached.is_some() {
        println!("hit {} ({:?})", action.id, started.elapsed());
        return Outcome::Green;
    }
    let logs = match super::store::output_path(execution.work, Path::new("target/ci/logs")) {
        Ok(path) => path,
        Err(error) => {
            action_failure(execution.work, action.id, &error);
            return Outcome::Broken;
        }
    };
    let outcome = match isolated_process(execution.view, prepared, &logs) {
        Ok(outcome) => outcome,
        Err(err) => {
            action_failure(execution.work, action.id, &err);
            Outcome::Broken
        }
    };
    if outcome == Outcome::Green
        && policy == Reuse::SuccessOnly
        && let Err(err) = store.save_green(action.id, &prepared.key, environment)
    {
        cache_warning(execution.work, action.id, "save", &err);
    }
    println!("{:?} {} ({:?})", outcome, action.id, started.elapsed());
    outcome
}

fn cache_warning(root: &Path, id: &str, operation: &str, error: &XtaskError) {
    Diagnostic {
        code: "verdict-cache-unavailable",
        action: &format!("{operation} a verification verdict"),
        subject: id,
        path: root,
        value: &error.to_string(),
        rule: "cache entries must remain in the configured output boundary",
        repair: "Run cargo xtask ci run --store target/ci/recovery-verdicts with a fresh output directory.",
    }
    .emit();
}

fn action_failure(root: &Path, id: &str, error: &XtaskError) {
    Diagnostic {
        code: "ci-action-execution",
        action: "execute the verification action and keep its log",
        subject: id,
        path: root,
        value: &error.to_string(),
        rule: "the action and its log must have usable destinations",
        repair: "Run just prereqs, restore target/ci/logs as a writable output directory, then rerun just verify-sources.",
    }
    .emit();
}

/// Only the explicitly audited pure gates read the identity-bearing view.
/// Every other action gets an independent ordinary-file copy, so even a tool
/// that rewrites its inputs cannot change another action's named bytes.
fn isolated_process(
    view: &Snapshot,
    prepared: &PreparedAction,
    logs: &Path,
) -> Result<Outcome, XtaskError> {
    let action = prepared.action;
    if action.reuse == Reuse::SuccessOnly {
        return process(view.root(), prepared, logs);
    }
    let workspace = Snapshot::capture(view.root())?;
    let result = process(workspace.root(), prepared, logs);
    workspace.finish(result)
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn process(root: &Path, prepared: &PreparedAction, logs: &Path) -> Result<Outcome, XtaskError> {
    let operation = prepared.action.operation;
    if let Operation::Gate(name) | Operation::ScopedGate(name) = operation {
        let scope = match operation {
            Operation::ScopedGate(_) => prepared.files.as_ref(),
            _ => None,
        };
        let code = crate::gates::run_scoped(root, &[name.to_owned()], scope);
        return Ok(if code == ExitCode::SUCCESS {
            Outcome::Green
        } else if code == ExitCode::FAILURE {
            Outcome::Refused
        } else {
            Outcome::Broken
        });
    }
    std::fs::create_dir_all(logs).map_err(|source| XtaskError::Io {
        path: logs.display().to_string(),
        source,
    })?;
    let (mut log, log_path) = tempfile::Builder::new()
        .prefix(prepared.action.id)
        .suffix(".log")
        .tempfile_in(logs)
        .and_then(|file| file.keep().map_err(|error| error.error))
        .map_err(|source| XtaskError::Io {
            path: logs.display().to_string(),
            source,
        })?;
    let log_error = |source| XtaskError::Io {
        path: log_path.display().to_string(),
        source,
    };
    let mut command = match operation {
        Operation::Gate(_) | Operation::ScopedGate(_) => {
            return Err(XtaskError::Doc {
                file: "verification dispatch (gate-dispatch)".to_owned(),
                msg: "source gates must use the in-process roster over the owned view".to_owned(),
            });
        }
        Operation::Just(recipe) => {
            let mut command = Command::new("just");
            #[cfg(windows)]
            command
                .arg("--shell")
                .arg(executable("bash")?)
                .arg("--shell-arg=-uc");
            command.arg(recipe);
            command
        }
        Operation::Process { program, args } => {
            let mut command = Command::new(executable(program)?);
            command.args(args);
            command
        }
    };
    let status = command
        .current_dir(root)
        .env("PYTHONUTF8", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .envs(&prepared.variables)
        .stdout(Stdio::from(log.try_clone().map_err(log_error)?))
        .stderr(Stdio::from(log.try_clone().map_err(log_error)?))
        .status()
        .map_err(log_error)?;
    if !status.success() {
        let mut output = Vec::new();
        match log.rewind().and_then(|()| log.read_to_end(&mut output)) {
            Ok(_) => eprintln!(
                "--- {} ---\n{}",
                prepared.action.id,
                String::from_utf8_lossy(&output)
            ),
            Err(source) => action_failure(root, prepared.action.id, &log_error(source)),
        }
    }
    Ok(match status.code() {
        Some(0) => Outcome::Green,
        Some(1) => Outcome::Refused,
        Some(_) | None => Outcome::Broken,
    })
}

/// Resolve the shell that runs repository scripts. On Windows, a bare
/// `bash` can select WSL; these scripts use the Git Bash shipped with the
/// development and Actions images.
fn executable(program: &str) -> Result<PathBuf, XtaskError> {
    #[cfg(windows)]
    if ["bash", "sh"].contains(&program) {
        return git_shell(program);
    }
    Ok(PathBuf::from(program))
}

#[cfg(windows)]
#[expect(
    clippy::disallowed_methods,
    reason = "developer tool Git shell discovery (child D4)"
)]
fn git_shell(program: &str) -> Result<PathBuf, XtaskError> {
    let output = Command::new("git")
        .arg("--exec-path")
        .output()
        .map_err(|source| XtaskError::Io {
            path: "git --exec-path (git-bash-missing)".to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(XtaskError::Cmd {
            cmd: "git --exec-path (git-bash-missing)".to_owned(),
            msg: "put Git for Windows on PATH and rerun verification".to_owned(),
        });
    }
    let path = String::from_utf8(output.stdout).map_err(|error| XtaskError::Doc {
        file: "Git exec-path (git-bash-path)".to_owned(),
        msg: format!(
            "{error}; install Git for Windows at a UTF-8 path and put its cmd directory on PATH"
        ),
    })?;
    for ancestor in Path::new(path.trim()).ancestors() {
        for directory in ["bin", "usr/bin"] {
            let candidate = ancestor.join(directory).join(format!("{program}.exe"));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(XtaskError::Cmd {
        cmd: format!("locate Git for Windows {program} (git-bash-missing)"),
        msg: "install Git for Windows with its Unix tools, put its cmd directory on PATH and rerun verification".to_owned(),
    })
}

fn report(outcomes: &BTreeMap<String, Outcome>) -> ExitCode {
    let mut refused = 0_usize;
    let mut broken = 0_usize;
    for outcome in outcomes.values().filter_map(|outcome| outcome.verdict()) {
        match outcome {
            Verdict::Green => {}
            Verdict::Refused => refused = refused.saturating_add(1),
            Verdict::Broken => broken = broken.saturating_add(1),
        }
    }
    if outcomes
        .values()
        .any(|outcome| *outcome == Outcome::Blocked)
    {
        refused = refused.saturating_add(1);
    }
    if broken > 0 {
        eprintln!("ci: {broken} action(s) could not execute");
        ExitCode::from(2)
    } else if refused > 0 {
        eprintln!("ci: {refused} action(s) refused the tree or were blocked");
        ExitCode::FAILURE
    } else {
        println!("ci: every selected action is green");
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod wire_tests;

#[cfg(all(test, unix))]
#[allow(clippy::unwrap_used, reason = "test code")]
mod output_tests;
