// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Content-addressed verification: the graph owns identity and selection;
//! storage and process execution live behind adapters (xtask D47).

mod baseline;
mod content;
mod diagnostic;
mod execute;
mod graph;
mod store;
mod suite;
mod view;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

use crate::report::XtaskError;
use content::Environment;
use diagnostic::Diagnostic;
use graph::Reuse;
use store::Store;

/// Plan or execute the source-verification graph (xtask D47).
pub(crate) fn run(root: &Path, args: &[String]) -> ExitCode {
    match parse(args).and_then(|options| match options.command {
        CommandKind::Plan => plan(root, &options),
        CommandKind::Run => execute(root, &options),
    }) {
        Ok(code) => code,
        Err(error) => {
            let (code, rule, repair) = match &error {
                XtaskError::CiUsage { code, .. } => (
                    *code,
                    "name plan or run and supply the value of each distinct option",
                    "Run cargo xtask ci plan, then add a named option and its value separately.",
                ),
                XtaskError::CiBaselineRevision { .. } => (
                    "ci-baseline-revision",
                    "the requested comparison revision must resolve to a Git tree",
                    "Run git fetch origin if the intended base is missing, or correct its spelling. Rerun the original command with the intended revision and the same options.",
                ),
                _ => (
                    "ci-verification-failed",
                    "capture a valid checkout and read its physical inputs",
                    "Run git status and git fsck in this checkout. Correct the reported input or output, then rerun the original command with the same options.",
                ),
            };
            Diagnostic {
                code,
                action: "plan or run source verification",
                subject: "xtask ci invocation",
                path: root,
                value: &error.to_string(),
                rule,
                repair,
            }
            .emit();
            ExitCode::from(2)
        }
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
    scope_base: Option<String>,
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
    let mut scope_base = None;
    let mut full = false;
    let mut store = None;
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--full" if !full && base.is_none() => {
                full = true;
            }
            "--base" if !full && base.is_none() && scope_base.is_none() => {
                base = Some(
                    it.next()
                        .ok_or_else(|| usage("--base needs a revision", "missing-base"))?
                        .clone(),
                );
            }
            "--scope-base" if base.is_none() && scope_base.is_none() => {
                scope_base = Some(
                    it.next()
                        .ok_or_else(|| {
                            usage("--scope-base needs a revision", "missing-scope-base")
                        })?
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
        scope_base,
        store,
    })
}

fn usage(detail: &str, code: &'static str) -> XtaskError {
    XtaskError::CiUsage {
        code,
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
    files: Option<Vec<String>>,
}

fn plan(root: &Path, options: &Options) -> Result<ExitCode, XtaskError> {
    let environment = Environment::current();
    let head = baseline::head(root)?;
    let snapshot = view::Snapshot::capture(root)?;
    let result = (|| {
        let changed = comparison(root, options, &snapshot)?;
        let files = changed.as_ref().map(crate::source::Scope::from_capture);
        let prepared = suite::prepare(&snapshot, &environment, &head, files.as_ref())?;
        let selected = selected(options, changed.as_ref(), &prepared)?;
        let rows: Vec<PlanRow> = prepared
            .iter()
            .map(|prepared| PlanRow {
                id: prepared.action.id,
                key: store::remote_key(prepared.action.id, &prepared.key, &environment),
                operation: operation_name(prepared.action.operation).to_owned(),
                needs: prepared.action.needs,
                reuse: match environment.reuse(prepared.action.reuse) {
                    Reuse::SuccessOnly => "success-only",
                    Reuse::Never => "never",
                },
                selected: selected.contains(prepared.action.id),
                files: prepared
                    .files
                    .as_ref()
                    .map(|scope| scope.paths().iter().cloned().collect()),
            })
            .collect();
        let text = serde_json::to_string_pretty(&rows).map_err(|err| XtaskError::Doc {
            file: "ci plan (plan-serialize)".to_owned(),
            msg: err.to_string(),
        })?;
        println!("{text}");
        Ok(ExitCode::SUCCESS)
    })();
    snapshot.finish(result)
}

fn execute(root: &Path, options: &Options) -> Result<ExitCode, XtaskError> {
    let store = Store::for_root(root, options.store.clone())?;
    let environment = Environment::current();
    let head = baseline::head(root)?;
    let snapshot = view::Snapshot::capture(root)?;
    let result = (|| {
        let changed = comparison(root, options, &snapshot)?;
        let files = changed.as_ref().map(crate::source::Scope::from_capture);
        let prepared = suite::prepare(&snapshot, &environment, &head, files.as_ref())?;
        let selected = selected(options, changed.as_ref(), &prepared)?;
        Ok(execute::run(
            &execute::Execution {
                view: &snapshot,
                work: root,
                store: &store,
                environment: &environment,
            },
            &prepared,
            &selected,
        ))
    })();
    snapshot.finish(result)
}

fn comparison(
    root: &Path,
    options: &Options,
    snapshot: &view::Snapshot,
) -> Result<Option<BTreeSet<String>>, XtaskError> {
    options
        .base
        .as_ref()
        .or(options.scope_base.as_ref())
        .map(|base| baseline::changed(root, base, snapshot))
        .transpose()
}

fn selected(
    options: &Options,
    changed: Option<&BTreeSet<String>>,
    prepared: &[suite::PreparedAction],
) -> Result<BTreeSet<String>, XtaskError> {
    if options.base.is_some() {
        let changed = changed.ok_or_else(|| {
            usage(
                "an affected graph requires its captured comparison",
                "missing-base-comparison",
            )
        })?;
        let actions: Vec<graph::Action> = prepared.iter().map(|row| *row.action).collect();
        Ok(graph::affected_with_base(
            &actions,
            suite::BASE_INPUTS,
            changed,
        ))
    } else {
        let actions: Vec<graph::Action> = prepared.iter().map(|row| *row.action).collect();
        Ok(graph::full(&actions))
    }
}

fn operation_name(operation: graph::Operation) -> String {
    match operation {
        graph::Operation::Gate(name) => format!("gate:{name}"),
        graph::Operation::ScopedGate(name) => format!("scoped-gate:{name}"),
        graph::Operation::Just(recipe) => format!("just:{recipe}"),
        graph::Operation::Process { program, args } => {
            format!("process:{program} {}", args.join(" "))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
