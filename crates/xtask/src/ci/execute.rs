// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Parallel execution with prerequisite propagation. Storage and process
//! failures never prevent unrelated nodes from reporting (xtask D43).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;

use crate::report::XtaskError;

use super::content::Environment;
use super::graph::{Operation, Reuse};
use super::store::{Store, Verdict};
use super::suite::PreparedAction;

/// The execution environment shared by every action in one run.
pub(crate) struct Execution<'a> {
    pub(crate) root: &'a Path,
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
    let root = execution.root;
    let store = execution.store;
    let environment = execution.environment;
    let action = prepared.action;
    let started = std::time::Instant::now();
    let cached = match store.restore(action.id, &prepared.key, environment, action.reuse) {
        Ok(cached) => cached,
        Err(err) => {
            eprintln!(
                "cache restore unavailable for {}; executing: {err}",
                action.id
            );
            None
        }
    };
    if cached.is_some() {
        println!("hit {} ({:?})", action.id, started.elapsed());
        return Outcome::Green;
    }
    let log = root
        .join("target/ci/logs")
        .join(format!("{}-{}.log", action.id, std::process::id()));
    let outcome = match process(root, action.operation, &log) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("{} could not execute: {err}", action.id);
            Outcome::Broken
        }
    };
    if outcome == Outcome::Green
        && action.reuse == Reuse::SuccessOnly
        && let Err(err) = store.save_green(action.id, &prepared.key, environment)
    {
        eprintln!("cache save unavailable for {}: {err}", action.id);
    }
    if outcome != Outcome::Green {
        match std::fs::read_to_string(&log) {
            Ok(text) => eprintln!("--- {} ---\n{text}", action.id),
            Err(err) => eprintln!("read the failed action log {}: {err}", log.display()),
        }
    }
    println!("{:?} {} ({:?})", outcome, action.id, started.elapsed());
    outcome
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn process(root: &Path, operation: Operation, log: &Path) -> Result<Outcome, XtaskError> {
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let output = std::fs::File::create(log).map_err(|source| XtaskError::Io {
        path: log.display().to_string(),
        source,
    })?;
    let errors = output.try_clone().map_err(|source| XtaskError::Io {
        path: log.display().to_string(),
        source,
    })?;
    let mut command = match operation {
        Operation::Gate(name) => {
            let executable = std::env::current_exe().map_err(|source| XtaskError::Io {
                path: "current xtask executable".to_owned(),
                source,
            })?;
            let mut command = Command::new(executable);
            command.args(["gates", name]);
            command
        }
        Operation::Just(recipe) => {
            let mut command = Command::new("just");
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
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(errors))
        .status()
        .map_err(|source| XtaskError::Io {
            path: log.display().to_string(),
            source,
        })?;
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
    if program == "bash" {
        let path = PathBuf::from("C:/Program Files/Git/bin/bash.exe");
        if path.is_file() {
            return Ok(path);
        }
        return Err(XtaskError::Cmd {
            cmd: "locate Git Bash for a repository script".to_owned(),
            msg: "C:/Program Files/Git/bin/bash.exe is absent (git-bash-missing); install Git for Windows or make the workflow use its explicit shell path".to_owned(),
        });
    }
    Ok(PathBuf::from(program))
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
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::ci::content::Key;
    use crate::ci::graph::Action;

    fn prepared(id: &'static str, needs: &'static [&'static str]) -> PreparedAction {
        PreparedAction {
            action: Box::leak(Box::new(Action {
                id,
                operation: Operation::Just(id),
                inputs: &[],
                needs,
                reuse: Reuse::Never,
                required: true,
            })),
            key: Key::named(id.to_owned()),
        }
    }

    #[test]
    fn a_failure_blocks_dependents_but_not_independent_work() {
        let graph = vec![
            prepared("base", &[]),
            prepared("dependent", &["base"]),
            prepared("independent", &[]),
        ];
        let ran = Mutex::new(Vec::new());
        let outcomes = schedule(
            &graph,
            &BTreeSet::from([
                "base".to_owned(),
                "dependent".to_owned(),
                "independent".to_owned(),
            ]),
            |action| {
                ran.lock().unwrap().push(action.action.id);
                if action.action.id == "base" {
                    Outcome::Refused
                } else {
                    Outcome::Green
                }
            },
        );
        assert_eq!(outcomes.get("base"), Some(&Outcome::Refused));
        assert_eq!(outcomes.get("dependent"), Some(&Outcome::Blocked));
        assert_eq!(outcomes.get("independent"), Some(&Outcome::Green));
        assert!(!ran.lock().unwrap().contains(&"dependent"));
        assert!(ran.lock().unwrap().contains(&"independent"));
    }

    #[test]
    fn an_empty_affected_scope_is_green_without_work() {
        let outcomes = schedule(&[], &BTreeSet::new(), |_| Outcome::Broken);
        assert_eq!(report(&outcomes), ExitCode::SUCCESS);
    }

    #[test]
    fn workflow_bash_never_resolves_to_the_wsl_name_on_windows() {
        let resolved = executable("bash").unwrap();
        #[cfg(windows)]
        assert_eq!(resolved, PathBuf::from("C:/Program Files/Git/bin/bash.exe"));
        #[cfg(not(windows))]
        assert_eq!(resolved, PathBuf::from("bash"));
    }
}
