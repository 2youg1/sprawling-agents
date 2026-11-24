// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::budget::REGISTER;
use crate::modmap;
use crate::report::{Violation, XtaskError};
use crate::walk;

mod measurement;
mod register;
mod zig;

pub(crate) const FILE_LINE_LIMIT: usize = 500;

use measurement::{Body, Found, measure};
use register::{ARG_ROW, PREDATING, ROW, excused, key, limit};

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    check_with(root, None)
}

pub(crate) fn check_scoped(
    root: &Path,
    scope: &crate::source::Scope,
) -> Result<Vec<Violation>, XtaskError> {
    check_with(root, Some(scope))
}

fn check_with(
    root: &Path,
    scope: Option<&crate::source::Scope>,
) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = source_files(root, scope)?;
    let body_limit = limit(root, ROW)?;
    let arg_limit = limit(root, ARG_ROW)?;
    let excused = excused(root)?;
    let shapes = modmap::shapes(root)?;
    let mut still_here: BTreeMap<String, usize> = BTreeMap::new();
    for file in sources(root)? {
        let rel = walk::rel(root, &file);
        if scope.is_some_and(|scope| !scope.allows(&rel)) {
            continue;
        }
        if shapes.get(&rel).is_some_and(|shape| shape == "data") {
            continue;
        }
        let text = walk::read_text(&file)?;
        let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
            file: rel.clone(),
            msg: format!("this file does not parse as Rust: {err}"),
        })?;
        for found in measure(&parsed.items) {
            if found.body.lines > body_limit {
                violations.push(over(&rel, &found.body, body_limit));
            }
            let named = key(&rel, &found.body.name);
            if excused.contains(&named) {
                let widest = still_here.entry(named).or_insert(found.args);
                *widest = (*widest).max(found.args);
            } else if found.args > arg_limit {
                violations.push(too_many_arguments(&rel, &found, arg_limit));
            }
        }
    }
    for file in zig_sources(root)? {
        let rel = walk::rel(root, &file);
        if scope.is_some_and(|scope| !scope.allows(&rel)) {
            continue;
        }
        if shapes.get(&rel).is_some_and(|shape| shape == "data") {
            continue;
        }
        let text = walk::read_text(&file)?;
        violations.extend(
            zig::measure(&text)
                .iter()
                .filter(|body| body.lines > body_limit)
                .map(|body| over(&rel, body, body_limit)),
        );
    }
    if scope.is_none() {
        violations.extend(spent_signatures(&excused, &still_here, arg_limit));
    }
    Ok(violations)
}

fn source_files(
    root: &Path,
    scope: Option<&crate::source::Scope>,
) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for file in crate::source::length_files_in(root, scope)? {
        let lines = walk::read_text(&file)?.lines().count();
        if lines > FILE_LINE_LIMIT {
            violations.push(Violation {
                gate: "length",
                location: walk::rel(root, &file),
                rule: format!(
                    "a Rust or TypeScript source file has at most {FILE_LINE_LIMIT} lines"
                ),
                violation: format!("the file has {lines} lines"),
                alternative: "rethink functional and architectural clarity; delete redundancy or split the file at a responsibility boundary".to_owned(),
            });
        }
    }
    Ok(violations)
}

fn spent_signatures(
    excused: &BTreeSet<String>,
    still_here: &BTreeMap<String, usize>,
    limit: usize,
) -> Vec<Violation> {
    let mut out = Vec::new();
    for named in excused {
        let violation = match still_here.get(named) {
            None => format!("{named} is excused and no longer here"),
            Some(&args) if args <= limit => {
                format!("{named} takes {args}, inside the {limit} the rule states")
            }
            Some(_) => continue,
        };
        out.push(Violation {
            gate: "length",
            location: format!("{REGISTER} [{ARG_ROW}.{PREDATING}]"),
            rule: "an exception that is no longer needed is struck from the register".to_owned(),
            violation,
            alternative: "delete its row: the values that travelled together have a name now, \
                          and a spent exception left in place re-excuses whatever is written \
                          at that address next"
                .to_owned(),
        });
    }
    out
}

fn too_many_arguments(rel: &str, found: &Found, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: format!("{rel}:{}", found.body.line),
        rule: format!(
            "a function takes at most {limit} parameters \
             ({REGISTER}, {ARG_ROW})"
        ),
        violation: format!("{} takes {}", found.body.name, found.args),
        alternative: "the ones that always travel together are one value: give them a struct \
                      with a name, as `Reporter` did for the four that describe who is \
                      reporting a change to a plan"
            .to_owned(),
    }
}

fn over(rel: &str, body: &Body, limit: usize) -> Violation {
    Violation {
        gate: "length",
        location: format!("{rel}:{}", body.line),
        rule: format!(
            "a production function stays inside {limit} lines \
             ({REGISTER}, function_length)"
        ),
        violation: format!("{} is {} lines", body.name, body.lines),
        alternative: "give a phase of it its own name, and hand the values it produces back \
                      as one value"
            .to_owned(),
    }
}

fn sources(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let mut out = Vec::new();
    for member in crate::members::members(root)? {
        let base = root.join(&member.dir).join("src");
        if base.is_dir() {
            out.extend(walk::files_with_ext(&base, &["rs"])?);
        }
    }
    Ok(out)
}

fn zig_sources(root: &Path) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let members = crate::members::members(root)?;
    walk::files_under(
        root,
        members.iter().map(|member| member.dir.as_str()),
        &["zig"],
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
