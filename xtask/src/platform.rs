// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which platforms a release builds for, and how each one is spelled.
//!
//! **One table, five readers.** The workflow that builds the archives,
//! the two install scripts that download one, the npm shim that execs
//! one, and the channel that repackages them all had to name the same
//! platforms, and each wrote its own list. Only two of the five were
//! ever compared, so `install.sh` came to offer a suffix the matrix
//! never produces — a download that answers 404 on the one platform
//! whose archive is built from a triple.
//!
//! The table below is the authority. The four files that restate it are
//! written in YAML, shell, PowerShell and JavaScript, so they are held
//! to it by reading them rather than by generating them: a gate does not
//! rewrite a workflow or an install script, it refuses one that disagrees
//! (`wiring` already reads real sources for the same reason).
//!
//! **What the reading reaches, and what it does not.** A matrix row and
//! a shim key are structured, so they are compared whole. A suffix built
//! by interpolation — `"-${os}-${arch}.zip"` — is not a spelling this
//! can compare, so only suffixes written out in full are judged. That is
//! an under-report and it is the affordable half: an archive name spelled
//! out wrongly is the defect that has happened, and an interpolated one
//! is refused by the download itself with the list of what the release
//! does carry.

use std::path::Path;

use crate::budget;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// One platform a release publishes a binary for, in every spelling the
/// tree needs: the archive's, the workflow's, npm's, and the file name
/// inside the archive.
pub(crate) struct Platform {
    /// What the release archive's name ends with.
    pub(crate) suffix: &'static str,
    /// The runner image `release.yml` builds this row on.
    pub(crate) runner: &'static str,
    /// The `--target` the matrix passes down, empty for a host build.
    pub(crate) cargo_target: &'static str,
    pub(crate) package: &'static str,
    /// npm's own spelling of the platform, which is what makes the
    /// install conditional: npm and bun skip an optional dependency
    /// whose `os`/`cpu` do not match, so one machine downloads one
    /// binary.
    pub(crate) os: &'static str,
    pub(crate) cpu: &'static str,
    pub(crate) binary: &'static str,
}

/// Every platform this project ships a binary for.
///
/// An archive with no row here is refused rather than skipped: the day a
/// Linux archive returns, the channel says so instead of publishing one
/// that quietly lacks it.
///
/// **These names carry the `@sprawling` scope, and the versions at or
/// below `UNSCOPED_THROUGH` do not.** npm never reuses a `name@version`,
/// so the rename cannot reach backwards: everything already on the
/// registry under a bare name stays there and is deprecated in place,
/// pointing at the scoped name that succeeds it. The root package keeps
/// its bare name whatever happens to these: `bunx sprawling` is the
/// whole reason that channel exists, and a scoped root would spell it
/// `bunx @sprawling/sprawling`.
pub(crate) const PLATFORMS: [Platform; 3] = [
    Platform {
        suffix: "-windows-x86_64.zip",
        runner: "windows-latest",
        cargo_target: "",
        package: "@sprawling/sprawling-windows-x64",
        os: "win32",
        cpu: "x64",
        binary: "sprawling.exe",
    },
    Platform {
        suffix: "-macos-aarch64.zip",
        runner: "macos-latest",
        cargo_target: "",
        package: "@sprawling/sprawling-darwin-arm64",
        os: "darwin",
        cpu: "arm64",
        binary: "sprawling",
    },
    Platform {
        suffix: "-x86_64-unknown-linux-musl.zip",
        runner: "ubuntu-latest",
        cargo_target: "x86_64-unknown-linux-musl",
        package: "@sprawling/sprawling-linux-x64-musl",
        os: "linux",
        cpu: "x64",
        binary: "sprawling",
    },
];

/// The workflow whose matrix builds one archive per row.
const WORKFLOW: &str = ".github/workflows/release.yml";

/// The shim that picks a package for the machine running it.
const SHIM: &str = "npm/shim.js";

/// Every file that writes an archive name out in full.
const SPELLERS: [&str; 3] = [WORKFLOW, "install.sh", "install.ps1"];

/// The register row that records the suffixes spelled before this rule.
const REGISTER_ROW: &str = "archive_naming";

/// Every place a foreign file restates this table still restates it.
///
/// # Errors
/// When a restating file or the budget register cannot be read.
pub(crate) fn restated(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = matrix_rows(root)?;
    violations.extend(shim_rows(root)?);
    violations.extend(spelled_suffixes(root)?);
    Ok(violations)
}

/// What the workflow's archive matrix builds, as `(runner, target)`.
///
/// Read by shape rather than by a YAML parser: a matrix entry opens with
/// `- os:` and states its `target:` on the next line, and this workflow
/// is the only reader of that shape. A matrix written some other way is
/// reported as a row this cannot find, never as agreement.
fn matrix(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(runner) = line.strip_prefix("- os:") else {
            continue;
        };
        let target = lines
            .get(index.saturating_add(1))
            .and_then(|next| next.strip_prefix("target:"))
            .unwrap_or_default();
        out.push((unquoted(runner), unquoted(target)));
    }
    out
}

/// A YAML or JavaScript scalar without its quotes or trailing comma.
fn unquoted(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(',')
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_owned()
}

fn matrix_rows(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let built = matrix(&walk::read_text(&root.join(WORKFLOW))?);
    let wanted: Vec<(String, String)> = PLATFORMS
        .iter()
        .map(|row| (row.runner.to_owned(), row.cargo_target.to_owned()))
        .collect();
    let mut out = Vec::new();
    for (runner, target) in &wanted {
        if !built.contains(&(runner.clone(), target.clone())) {
            out.push(disagrees(
                WORKFLOW,
                format!("no matrix row builds `{runner}` for target `{target}`"),
            ));
        }
    }
    for pair in &built {
        if !wanted.contains(pair) {
            out.push(disagrees(
                WORKFLOW,
                format!(
                    "a matrix row builds `{}` for target `{}`, which no platform claims",
                    pair.0, pair.1
                ),
            ));
        }
    }
    Ok(out)
}

/// What the shim hands a machine: the key it looks the machine up by,
/// the package it resolves, and the file it execs.
fn shim(text: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some((key, rest)) = trimmed.strip_prefix('"').and_then(|l| l.split_once("\":")) else {
            continue;
        };
        let Some(package) = field(rest, "package:") else {
            continue;
        };
        let Some(binary) = field(rest, "binary:") else {
            continue;
        };
        out.push((key.to_owned(), package, binary));
    }
    out
}

/// One `name: "value"` out of an object literal written on one line.
fn field(text: &str, key: &str) -> Option<String> {
    let after = text.split_once(key)?.1.trim_start();
    let inner = after.strip_prefix('"')?;
    let (value, _) = inner.split_once('"')?;
    Some(value.to_owned())
}

fn shim_rows(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let held = shim(&walk::read_text(&root.join(SHIM))?);
    let wanted: Vec<(String, String, String)> = PLATFORMS
        .iter()
        .map(|row| {
            (
                format!("{} {}", row.os, row.cpu),
                row.package.to_owned(),
                row.binary.to_owned(),
            )
        })
        .collect();
    let mut out = Vec::new();
    for row in &wanted {
        if !held.contains(row) {
            out.push(disagrees(
                SHIM,
                format!("no entry maps `{}` to `{}` and `{}`", row.0, row.1, row.2),
            ));
        }
    }
    for row in &held {
        if !wanted.contains(row) {
            out.push(disagrees(
                SHIM,
                format!("`{}` resolves `{}`, which no platform claims", row.0, row.1),
            ));
        }
    }
    Ok(out)
}

/// Every archive suffix written out in full names an archive a release
/// publishes, unless the register still records it.
fn spelled_suffixes(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let recorded = registered(root)?;
    let mut out = Vec::new();
    let mut met: Vec<String> = Vec::new();
    for rel in SPELLERS {
        let path = root.join(rel);
        if !path.is_file() {
            continue;
        }
        for (number, line) in walk::read_text(&path)?.lines().enumerate() {
            for suffix in suffixes(line) {
                met.push(suffix.clone());
                if PLATFORMS.iter().any(|row| row.suffix == suffix) {
                    continue;
                }
                if recorded.contains(&suffix) {
                    continue;
                }
                out.push(disagrees(
                    &format!("{rel}:{}", number.saturating_add(1)),
                    format!("`{suffix}` ends no archive this release builds"),
                ));
            }
        }
    }
    out.extend(spent_suffixes(&recorded, &met));
    Ok(out)
}

/// The register rows that no longer record anything.
///
/// Same discipline as the two length registers and `boundary`'s: a
/// recorded exception that has become unnecessary is struck, so the
/// register removes itself rather than waiting to be remembered.
fn spent_suffixes(recorded: &[String], met: &[String]) -> Vec<Violation> {
    let mut out = Vec::new();
    for suffix in recorded {
        let violation = if PLATFORMS.iter().any(|row| row.suffix == *suffix) {
            format!("`{suffix}` is an archive this release now builds")
        } else if !met.contains(suffix) {
            format!("`{suffix}` is recorded and nobody spells it")
        } else {
            continue;
        };
        out.push(Violation {
            gate: "artifact",
            location: format!("xtask/budgets.toml [{REGISTER_ROW}.predating]"),
            rule: "an exception that is no longer needed is struck from the register".to_owned(),
            violation,
            alternative: "delete its row: a spent exception left in place re-excuses whatever \
                          is written at that address next"
                .to_owned(),
        });
    }
    out
}

/// The suffixes the register still records.
fn registered(root: &Path) -> Result<Vec<String>, XtaskError> {
    let register = budget::register(root)?;
    let Some(listed) = register
        .get(REGISTER_ROW)
        .and_then(|row| row.get("predating"))
        .and_then(|table| table.get("suffixes"))
        .and_then(toml::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for value in listed {
        let suffix = value.as_str().ok_or_else(|| XtaskError::Doc {
            file: "xtask/budgets.toml".to_owned(),
            msg: format!("{REGISTER_ROW}.predating holds something that is not a suffix"),
        })?;
        out.push(suffix.to_owned());
    }
    Ok(out)
}

/// Every archive suffix one line writes out in full.
///
/// A suffix is a run of name characters that opens with `-` and closes
/// with `.zip`. An interpolated name breaks the run at its `$`, so it
/// arrives here as `.zip` and is dropped: what cannot be compared is not
/// guessed at.
fn suffixes(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in line.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-') {
            current.push(ch);
            continue;
        }
        let token = std::mem::take(&mut current);
        if token.starts_with('-') && token.ends_with(".zip") {
            out.push(token);
        }
    }
    out
}

fn disagrees(location: &str, violation: String) -> Violation {
    Violation {
        gate: "artifact",
        location: location.to_owned(),
        rule: "every platform a release builds for is spelled the way `xtask::platform` \
               spells it"
            .to_owned(),
        violation,
        alternative: "bring this file to the table in `xtask/src/platform.rs`, or change that \
                      table and every file that restates it in one change-set"
            .to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
