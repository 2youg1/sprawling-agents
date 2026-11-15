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
    let store = execution.store;
    let environment = execution.environment;
    let action = prepared.action;
    let policy = environment.reuse(action.reuse);
    let started = std::time::Instant::now();
    let cached = match store.restore(action.id, &prepared.key, environment, policy) {
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
    let log = execution.work.join("target/ci/logs").join(format!(
        "{}-{}.log",
        action.id,
        std::process::id()
    ));
    let outcome = match isolated_process(execution.view, prepared, &log) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("{} could not execute: {err}", action.id);
            Outcome::Broken
        }
    };
    if outcome == Outcome::Green
        && policy == Reuse::SuccessOnly
        && let Err(err) = store.save_green(action.id, &prepared.key, environment)
    {
        eprintln!("cache save unavailable for {}: {err}", action.id);
    }
    if outcome != Outcome::Green && !matches!(action.operation, Operation::Gate(_)) {
        match std::fs::read_to_string(&log) {
            Ok(text) => eprintln!("--- {} ---\n{text}", action.id),
            Err(err) => eprintln!("read the failed action log {}: {err}", log.display()),
        }
    }
    println!("{:?} {} ({:?})", outcome, action.id, started.elapsed());
    outcome
}

/// Only the explicitly audited pure gates read the identity-bearing view.
/// Every other action gets an independent ordinary-file copy, so even a tool
/// that rewrites its inputs cannot change another action's named bytes.
fn isolated_process(
    view: &Snapshot,
    prepared: &PreparedAction,
    log: &Path,
) -> Result<Outcome, XtaskError> {
    let action = prepared.action;
    if action.reuse == Reuse::SuccessOnly {
        return process(view.root(), action.operation, &prepared.variables, log);
    }
    let workspace = Snapshot::capture(view.root())?;
    let result = process(workspace.root(), action.operation, &prepared.variables, log);
    workspace.finish(result)
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn process(
    root: &Path,
    operation: Operation,
    variables: &BTreeMap<String, String>,
    log: &Path,
) -> Result<Outcome, XtaskError> {
    if let Operation::Gate(name) = operation {
        let code = crate::gates::run(root, &[name.to_owned()]);
        return Ok(if code == ExitCode::SUCCESS {
            Outcome::Green
        } else if code == ExitCode::FAILURE {
            Outcome::Refused
        } else {
            Outcome::Broken
        });
    }
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
        Operation::Gate(_) => {
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
        .envs(variables)
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
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::ci::content::Key;
    use crate::ci::content::{self, Identity};
    use crate::ci::graph::Action;
    use crate::ci::graph::Input;

    #[test]
    fn a_later_good_tree_cannot_publish_green_for_a_captured_bad_tree() {
        let original = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(original.path().join("src")).unwrap();
        let file = original.path().join("src/lib.rs");
        let bad = "fn captured() {}\n";
        std::fs::write(&file, bad).unwrap();
        let view = Snapshot::capture(original.path()).unwrap();
        static HEADER: Action = Action {
            id: "header",
            operation: Operation::Gate("header"),
            inputs: &[Input::only("", &["rs"])],
            needs: &[],
            reuse: Reuse::SuccessOnly,
            required: true,
        };
        let environment = Environment {
            os: "test".to_owned(),
            arch: "test".to_owned(),
            verifier: Some("test-verifier".to_owned()),
        };
        let key = content::key_in_snapshot(
            &view,
            &HEADER,
            &Identity {
                base: &[],
                dependencies: &BTreeMap::new(),
                environment: &environment,
                variables: &BTreeMap::new(),
            },
        )
        .unwrap();
        let good = crate::header::notice(crate::header::Leader::Rust)
            .into_iter()
            .chain(["".to_owned(), "fn captured() {}".to_owned()])
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&file, good).unwrap();
        let good_view = Snapshot::capture(original.path()).unwrap();
        let good_key = content::key_in_snapshot(
            &good_view,
            &HEADER,
            &Identity {
                base: &[],
                dependencies: &BTreeMap::new(),
                environment: &environment,
                variables: &BTreeMap::new(),
            },
        )
        .unwrap();
        let store = Store::local(original.path().join("target/verdicts"));
        assert_ne!(
            key, good_key,
            "A and B must have distinct content identities"
        );
        let outcome = execute_one(
            &Execution {
                view: &view,
                work: original.path(),
                store: &store,
                environment: &environment,
            },
            &PreparedAction {
                action: &HEADER,
                key: key.clone(),
                variables: BTreeMap::new(),
            },
        );
        std::fs::write(&file, bad).unwrap();
        let cached = store
            .restore("header", &key, &environment, Reuse::SuccessOnly)
            .unwrap();
        let good_outcome = execute_one(
            &Execution {
                view: &good_view,
                work: original.path(),
                store: &store,
                environment: &environment,
            },
            &PreparedAction {
                action: &HEADER,
                key: good_key.clone(),
                variables: BTreeMap::new(),
            },
        );
        let good_cached = store
            .restore("header", &good_key, &environment, Reuse::SuccessOnly)
            .unwrap();
        let restored_view = Snapshot::capture(original.path()).unwrap();
        let restored_key = content::key_in_snapshot(
            &restored_view,
            &HEADER,
            &Identity {
                base: &[],
                dependencies: &BTreeMap::new(),
                environment: &environment,
                variables: &BTreeMap::new(),
            },
        )
        .unwrap();
        assert_eq!(
            restored_key, key,
            "a fresh capture of restored A must recover A's identity"
        );
        let restored_outcome = execute_one(
            &Execution {
                view: &restored_view,
                work: original.path(),
                store: &store,
                environment: &environment,
            },
            &PreparedAction {
                action: &HEADER,
                key: restored_key,
                variables: BTreeMap::new(),
            },
        );
        assert_eq!(restored_outcome, Outcome::Refused);
        restored_view.close().unwrap();
        good_view.close().unwrap();
        view.close().unwrap();
        assert_eq!(
            (outcome, cached.is_some()),
            (Outcome::Refused, false),
            "the executor must check captured A and must not save B's green under A's key"
        );
        assert_eq!(good_outcome, Outcome::Green);
        assert!(
            good_cached.is_some(),
            "the controlled fixture identity permits real green reuse"
        );
        assert!(
            store
                .restore("header", &key, &environment, Reuse::SuccessOnly)
                .unwrap()
                .is_none(),
            "saving B must not populate A's key"
        );
    }

    #[test]
    fn failed_loaded_image_probe_still_executes_the_real_gate() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("bad.rs"), "fn missing_header() {}\n").unwrap();
        let view = Snapshot::capture(root.path()).unwrap();
        let environment = Environment::from_probe(Err(XtaskError::Io {
            path: "fixture loaded-image probe".to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "probe unavailable"),
        }));
        assert_eq!(environment.reuse(Reuse::SuccessOnly), Reuse::Never);
        static HEADER: Action = Action {
            id: "header",
            operation: Operation::Gate("header"),
            inputs: &[],
            needs: &[],
            reuse: Reuse::SuccessOnly,
            required: true,
        };
        let store = Store::local(root.path().join("target/verdicts"));
        let outcome = execute_one(
            &Execution {
                view: &view,
                work: root.path(),
                store: &store,
                environment: &environment,
            },
            &PreparedAction {
                action: &HEADER,
                key: Key::named("fixture".to_owned()),
                variables: BTreeMap::new(),
            },
        );
        assert_eq!(
            outcome,
            Outcome::Refused,
            "probe failure must not prevent the real bad-header verdict"
        );
        assert!(!root.path().join("target/verdicts").exists());
        view.close().unwrap();
        root.close().unwrap();
    }

    #[test]
    fn a_mutating_non_reusable_process_cannot_rewrite_pure_inputs() {
        let original = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(original.path().join("src")).unwrap();
        std::fs::write(original.path().join("src/lib.rs"), "captured A").unwrap();
        let view = Snapshot::capture(original.path()).unwrap();
        static MUTATION: Action = Action {
            id: "mutating-tool",
            operation: Operation::Process {
                program: "bun",
                args: &["-e", "await Bun.write('src/lib.rs', 'tool B')"],
            },
            inputs: &[],
            needs: &[],
            reuse: Reuse::Never,
            required: true,
        };
        let outcome = isolated_process(
            &view,
            &PreparedAction {
                action: &MUTATION,
                key: Key::named("mutation-fixture".to_owned()),
                variables: BTreeMap::new(),
            },
            &original.path().join("target/process.log"),
        )
        .unwrap();
        assert_eq!(outcome, Outcome::Green);
        assert_eq!(
            std::fs::read_to_string(view.root().join("src/lib.rs")).unwrap(),
            "captured A"
        );
        assert_eq!(crate::header::check(view.root()).unwrap().len(), 1);
        view.close().unwrap();
    }

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
            variables: BTreeMap::new(),
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
        {
            assert!(resolved.is_absolute() && resolved.is_file());
            assert_eq!(
                resolved.file_name().and_then(|name| name.to_str()),
                Some("bash.exe")
            );
        }
        #[cfg(not(windows))]
        assert_eq!(resolved, PathBuf::from("bash"));
    }
}
