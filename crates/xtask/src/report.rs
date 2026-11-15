// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Three-part refusal: rule | violation | alternative (xtask Spec §10-8).

use std::process::ExitCode;

/// One gate finding.
#[derive(Debug)]
pub(crate) struct Violation {
    pub(crate) gate: &'static str,
    pub(crate) location: String,
    pub(crate) rule: String,
    pub(crate) violation: String,
    pub(crate) alternative: String,
}

/// Gate-internal failure; exits 2, never silent pass or false judgement.
#[derive(Debug, thiserror::Error)]
pub(crate) enum XtaskError {
    #[error("io on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{file}: {msg}")]
    Doc { file: String, msg: String },
    #[error("command `{cmd}` failed: {msg}")]
    Cmd { cmd: String, msg: String },
    #[error("run gate `{name}`: no gate has that name (unknown-gate); name one of: {known}")]
    UnknownGate { name: String, known: String },
    #[error(
        "run gate `{name}`: the gate panicked with {message:?} (gate-panicked); run `cargo xtask gates {name}` to see the panic alone"
    )]
    GatePanicked { name: &'static str, message: String },
    #[error(
        "run gate `{name}` with --files: this gate has no file-scoped judgement (unsupported-file-scope); select only `length` and/or `comments`, or omit --files for a full-tree judgement"
    )]
    UnsupportedFileScope { name: &'static str },
    #[error(
        "read --files path `{path}`: the path is not repository-relative (invalid-file-scope); pass paths from `git diff --name-only`, without absolute paths or `..`"
    )]
    InvalidFileScope { path: String },
    #[error(
        "judge tree {here}: this xtask was built from {built} (stale-build); run `cargo clean -p xtask` and the command again, so cargo rebuilds it from this tree"
    )]
    StaleBuild { built: String, here: String },
    #[error(
        "judge tree: no directory from {from} upward holds Cargo.lock (no-checkout); run the command from inside the checkout it should judge"
    )]
    NoCheckout { from: String },
    #[error(
        "read package {package}: it sits at {dir}, outside the checkout {root} (member-outside-checkout); move the package under the checkout, or drop it from the workspace and from every path dependency"
    )]
    OutsideCheckout {
        package: String,
        dir: String,
        root: String,
    },
    #[error(
        "read package {package}: `[package.metadata.sprawling] role` is {role}, and the only role is \"tool\" (unknown-role); write role = \"tool\", or delete the line to make the package part of the product"
    )]
    UnknownRole { package: String, role: String },
    #[error(
        "find package `{name}`: no package in this workspace has that package or lib name (unknown-package); name one of: {known}"
    )]
    UnknownPackage { name: String, known: String },
    #[error(
        "read theme {entry}: it imports {part}, and no such file exists (theme-part-missing); create the part, or delete its `@import` line from {entry}"
    )]
    ThemePartMissing { entry: String, part: String },
    #[error(
        "read theme {entry}: {part} sits among the theme's parts and {entry} imports it nowhere (theme-part-unimported); import it from {entry} at the place its rules belong in the cascade, or delete it"
    )]
    ThemePartUnimported { entry: String, part: String },
}

pub(crate) fn render(violations: &[Violation]) {
    for v in violations {
        println!("[{}] {}", v.gate, v.location);
        println!("  rule:        {}", v.rule);
        println!("  violation:   {}", v.violation);
        println!("  alternative: {}", v.alternative);
    }
}

/// Render a single gate's outcome and map it to an exit code.
pub(crate) fn finish(gate: &'static str, result: Result<Vec<Violation>, XtaskError>) -> ExitCode {
    match result {
        Ok(violations) if violations.is_empty() => {
            println!("gate {gate}: ok");
            ExitCode::SUCCESS
        }
        Ok(violations) => {
            render(&violations);
            println!("gate {gate}: {} violation(s)", violations.len());
            ExitCode::FAILURE
        }
        Err(err) => internal_failure(&err),
    }
}

/// Render a whole run. A gate that could not judge does not stop the
/// walk (xtask D2): every gate is evaluated before the first line prints.
pub(crate) fn finish_all(
    results: impl IntoIterator<Item = (&'static str, Result<Vec<Violation>, XtaskError>)>,
) -> ExitCode {
    let mut findings = Vec::new();
    let mut broken = 0_usize;
    for (gate, result) in results {
        match result {
            Ok(violations) if violations.is_empty() => println!("gate {gate}: ok"),
            Ok(violations) => {
                println!("gate {gate}: {} violation(s)", violations.len());
                findings.extend(violations);
            }
            Err(err) => {
                println!("gate {gate}: could not judge");
                say_internal_failure(&err);
                broken = broken.saturating_add(1);
            }
        }
    }
    render(&findings);
    if !findings.is_empty() {
        println!("{} violation(s) across all gates", findings.len());
    }
    let outcome = RunOutcome::of(broken, findings.len());
    match outcome {
        RunOutcome::Green => println!("all gates green"),
        RunOutcome::Refused => {}
        RunOutcome::Broken => println!("{broken} gate(s) could not judge"),
    }
    outcome.code()
}

/// Run conclusion. Exit 2 (gate broken) outranks exit 1 (tree broken).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunOutcome {
    Green,
    Refused,
    Broken,
}

impl RunOutcome {
    fn of(broken: usize, findings: usize) -> Self {
        if broken > 0 {
            Self::Broken
        } else if findings > 0 {
            Self::Refused
        } else {
            Self::Green
        }
    }

    fn code(self) -> ExitCode {
        match self {
            Self::Green => ExitCode::SUCCESS,
            Self::Refused => ExitCode::FAILURE,
            Self::Broken => ExitCode::from(2),
        }
    }
}

pub(crate) fn internal_failure(err: &XtaskError) -> ExitCode {
    say_internal_failure(err);
    ExitCode::from(2)
}

/// The one wording for "a gate could not judge", so the single-gate path
/// and the whole-run path cannot describe the same event differently.
fn say_internal_failure(err: &XtaskError) {
    eprintln!("xtask internal failure: {err}");
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::RunOutcome;

    #[test]
    fn a_broken_gate_outranks_a_violation() {
        assert_eq!(RunOutcome::of(1, 7), RunOutcome::Broken);
        assert_eq!(RunOutcome::of(1, 0), RunOutcome::Broken);
        assert_eq!(RunOutcome::of(0, 7), RunOutcome::Refused);
        assert_eq!(RunOutcome::of(0, 0), RunOutcome::Green);
    }
}
