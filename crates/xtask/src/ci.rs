// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Content-addressed verification: the graph owns identity and selection;
//! storage and process execution live behind adapters (xtask D43).

mod content;
mod execute;
mod graph;
mod store;
mod suite;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Serialize;

use crate::report::{self, XtaskError};
use content::Environment;
use graph::Reuse;
use store::Store;

/// Plan or execute the source-verification graph (xtask D43).
pub(crate) fn run(root: &Path, args: &[String]) -> ExitCode {
    match parse(args).and_then(|options| match options.command {
        CommandKind::Plan => plan(root, &options),
        CommandKind::Run => execute(root, &options),
    }) {
        Ok(code) => code,
        Err(err) => report::internal_failure(&err),
    }
}

#[derive(Debug, Clone, Copy)]
enum CommandKind {
    Plan,
    Run,
}

#[derive(Debug)]
struct Options {
    command: CommandKind,
    base: Option<String>,
    store: Option<PathBuf>,
}

fn parse(args: &[String]) -> Result<Options, XtaskError> {
    let command = match args.first().map(String::as_str) {
        Some("plan") => CommandKind::Plan,
        Some("run") => CommandKind::Run,
        _ => {
            return Err(usage("name `plan` or `run`", "missing-ci-command"));
        }
    };
    let mut base = None;
    let mut store = None;
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--full" if base.is_none() => {}
            "--base" if base.is_none() => {
                base = Some(
                    it.next()
                        .ok_or_else(|| usage("--base needs a revision", "missing-base"))?
                        .clone(),
                );
            }
            "--store" if store.is_none() => {
                store = Some(PathBuf::from(it.next().ok_or_else(|| {
                    usage("--store needs a directory", "missing-store")
                })?));
            }
            _ => {
                return Err(usage(
                    &format!("unknown or repeated option {arg}"),
                    "invalid-ci-option",
                ));
            }
        }
    }
    Ok(Options {
        command,
        base,
        store,
    })
}

fn usage(detail: &str, code: &'static str) -> XtaskError {
    XtaskError::Doc {
        file: format!("cargo xtask ci <plan|run> [--full | --base <rev>] [--store <dir>] ({code})"),
        msg: detail.to_owned(),
    }
}

#[derive(Serialize)]
struct PlanRow {
    id: &'static str,
    key: String,
    operation: String,
    needs: &'static [&'static str],
    reuse: &'static str,
    selected: bool,
}

fn plan(root: &Path, options: &Options) -> Result<ExitCode, XtaskError> {
    let environment = Environment::current();
    let prepared = suite::prepare(root, &environment)?;
    let selected = selected(root, options, &prepared)?;
    let rows: Vec<PlanRow> = prepared
        .iter()
        .map(|prepared| PlanRow {
            id: prepared.action.id,
            key: store::remote_key(prepared.action.id, &prepared.key, &environment),
            operation: operation_name(prepared.action.operation).to_owned(),
            needs: prepared.action.needs,
            reuse: match prepared.action.reuse {
                Reuse::SuccessOnly => "success-only",
                Reuse::Never => "never",
            },
            selected: selected.contains(prepared.action.id),
        })
        .collect();
    let text = serde_json::to_string_pretty(&rows).map_err(|err| XtaskError::Doc {
        file: "ci plan (plan-serialize)".to_owned(),
        msg: err.to_string(),
    })?;
    println!("{text}");
    Ok(ExitCode::SUCCESS)
}

fn execute(root: &Path, options: &Options) -> Result<ExitCode, XtaskError> {
    let environment = Environment::current();
    let prepared = suite::prepare(root, &environment)?;
    let selected = selected(root, options, &prepared)?;
    let store = Store::for_root(root, options.store.clone());
    Ok(execute::run(
        &execute::Execution {
            root,
            store: &store,
            environment: &environment,
        },
        &prepared,
        &selected,
    ))
}

fn selected(
    root: &Path,
    options: &Options,
    prepared: &[suite::PreparedAction],
) -> Result<BTreeSet<String>, XtaskError> {
    if let Some(base) = &options.base {
        let changed = changed(root, base)?;
        let actions: Vec<graph::Action> = prepared.iter().map(|row| *row.action).collect();
        Ok(graph::affected_with_base(
            &actions,
            suite::BASE_INPUTS,
            &changed,
        ))
    } else {
        let actions: Vec<graph::Action> = prepared.iter().map(|row| *row.action).collect();
        Ok(graph::full(&actions))
    }
}

fn operation_name(operation: graph::Operation) -> String {
    match operation {
        graph::Operation::Gate(name) => format!("gate:{name}"),
        graph::Operation::Just(recipe) => format!("just:{recipe}"),
        graph::Operation::Process { program, args } => {
            format!("process:{program} {}", args.join(" "))
        }
    }
}

fn changed(root: &Path, base: &str) -> Result<BTreeSet<String>, XtaskError> {
    let tracked = git(root, &["diff", "--name-only", "--no-renames", base, "--"])?;
    let untracked = git(root, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(tracked
        .into_iter()
        .chain(untracked)
        .filter(|path| !crate::walk::in_isolation_zone(path))
        .collect())
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn git(root: &Path, args: &[&str]) -> Result<Vec<String>, XtaskError> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|source| XtaskError::Io {
            path: format!("git {}", args.join(" ")),
            source,
        })?;
    if !output.status.success() {
        return Err(XtaskError::Cmd {
            cmd: format!("git {}", args.join(" ")),
            msg: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    String::from_utf8(output.stdout)
        .map(|text| text.lines().map(str::to_owned).collect())
        .map_err(|err| XtaskError::Doc {
            file: format!("git {}", args.join(" ")),
            msg: format!("paths are not UTF-8: {err} (non-utf8-path)"),
        })
}
