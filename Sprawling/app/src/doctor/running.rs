// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one place this binary starts an install program, under a
//! patience it cannot outlive (`crates/sprawling/spec/Doctor.lean` §8-64).
//!
//! **A wait here is always bounded, and a child here can never wait on
//! a person.** The three facts that make that true are in `run` and
//! nowhere else: stdin is `Stdio::null()`, so a
//! package manager asking for a source agreement or a password reads
//! end of input and fails at once rather than sitting on a terminal
//! nobody is attending; the wait is a poll of `try_wait` against a
//! counted set of knocks rather than a blocking `Command::status`; and
//! running out of knocks kills the child and returns, so the caller's
//! thread goes back to its queue instead of waiting for a process that
//! has stopped making progress.
//!
//! **The wait is counted, not timed.** Reading a clock happens in one
//! module of this binary and this is not it (ARCHITECTURE.md section 10,
//! rule 4), so the bound is a number of knocks at a fixed interval.
//! That also makes the bound assertable: a test asks for three knocks
//! and gets three, where a test against a wall clock would be asking
//! the machine it runs on.
//!
//! That matters because the caller is the city's single writer thread:
//! a wait without an end there stops `Halt` and `Cancel` from being
//! read at all, which is the one moment a person most needs them.
//!
//! What the installer prints goes to the item's log file, both streams
//! into one, because a failure the person cannot read is a failure
//! they can only retry blind.
//!
//! `Runnable` is the recipe that passed the "may this city run it"
//! question, so nothing below re-asks it: an unrunnable recipe cannot
//! be spelled as this type (`Recipe::command`).

use std::path::Path;
use std::process::{Child, Stdio};
use std::time::Duration;

use accounting::Runnable;
use kernel::{AxCode, AxError};

/// How many times one install program that downloads a finished
/// package is asked whether it has finished before this city stops it.
///
/// `PATIENCE * TICK` is three minutes: long enough for a package
/// manager to download a toolchain over a slow link, and short enough
/// that a writer thread held that long still reads as a city at work.
pub(crate) const PATIENCE: u32 = 3_600;

/// The knocks a `cargo install` gets, which compiles from source:
/// `BUILD_PATIENCE * TICK` is twenty minutes, because cargo-mutants or
/// cargo-deny compile for five to twelve minutes cold on four cores and
/// three minutes killed every one of them half way. The writer thread is
/// held that long; the page draws the item as installing meanwhile, and
/// moving installs off that thread is the step `crates/sprawling/spec/Doctor.lean` §8-64
/// names as left.
const BUILD_PATIENCE: u32 = 24_000;

/// How long this install program may run: a compile from source gets
/// the build's patience, a download gets the download's.
pub(crate) fn patience_for(runnable: &Runnable) -> u32 {
    match runnable.program() {
        "cargo" => BUILD_PATIENCE,
        _ => PATIENCE,
    }
}

/// How long this city waits between two knocks. Short enough that a
/// fast install is not padded, long enough that three minutes of
/// waiting costs a few thousand cheap syscalls rather than a spinning
/// CPU.
const TICK: Duration = Duration::from_millis(50);

/// Runs one install program to completion, or stops it when the
/// knocks run out.
///
/// `item` names the requirement being installed and appears in every
/// refusal, because the person reading it is looking at a page listing
/// items rather than programs.
///
/// # Errors
/// Reports a program that will not start, a program that ended in
/// failure, and a program still running at the deadline. The last one
/// carries the recovery that actually works: run the line yourself,
/// where an installer that wants an answer can get one.
pub(crate) fn run(
    item: &str,
    runnable: &Runnable,
    patience: u32,
    log: &Path,
) -> Result<(), AxError> {
    let (stdout, stderr) = log_streams(item, log)?;
    let mut child = child::command(runnable.program())
        .args(runnable.args())
        .env("PATH", super::host::search_path())
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "install a tool",
                format!("{item}: {}: {err}", runnable.program()),
            )
            .with_recovery("install the package manager first, or run the printed line yourself")
        })?;
    let mut knocks = patience;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_ended_in_failure)) => {
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "install a tool",
                    format!("{item}: {} ended in failure", runnable.spelled()),
                )
                .with_recovery(format!(
                    "read what it reported in {}, then run the line yourself in a terminal",
                    log.display()
                )));
            }
            Ok(None) => {}
            Err(err) => {
                return Err(unwatchable(item, runnable, &stop(&mut child), &err));
            }
        }
        let Some(left) = knocks.checked_sub(1) else {
            let waited = Waited { patience, log };
            return Err(overran(item, runnable, &waited, &stop(&mut child)));
        };
        knocks = left;
        std::thread::sleep(TICK);
    }
}

/// A fresh log file for one install, opened once and handed to the
/// child as both of its output streams.
///
/// # Errors
/// Reports a log that cannot be created; the install does not start
/// without one, because a failure nobody can read is what it replaces.
fn log_streams(item: &str, log: &Path) -> Result<(Stdio, Stdio), AxError> {
    let refused = |err: std::io::Error| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "install a tool",
            format!(
                "{item}: its log {} could not be written: {err}",
                log.display()
            ),
        )
        .with_recovery("free the temporary directory, or run the line yourself in a terminal")
    };
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).map_err(refused)?;
    }
    let file = std::fs::File::create(log).map_err(refused)?;
    let second = file.try_clone().map_err(refused)?;
    Ok((Stdio::from(file), Stdio::from(second)))
}

/// Ends the child and says what went wrong while ending it.
///
/// `None` means the process is gone. A kill or a reap that fails is
/// carried into the caller's message rather than dropped: a process
/// this city started and could not stop is a fact the person needs,
/// because it is still holding whatever it was holding.
///
/// **Every child this binary starts ends here**, the install program
/// and the `--version` call alike (`doctor::probe::ask_version`), so
/// what "stopped" means is decided once.
pub(super) fn stop(child: &mut Child) -> Option<String> {
    if let Err(err) = child.kill() {
        return Some(format!("it could not be stopped: {err}"));
    }
    match child.wait() {
        Ok(_reaped) => None,
        Err(err) => Some(format!("it was stopped but not reaped: {err}")),
    }
}

/// How long an install was given, and where what it printed meanwhile
/// went: the two facts a person reads when the knocks ran out.
struct Waited<'a> {
    patience: u32,
    log: &'a Path,
}

/// The knocks ran out with the program still running.
fn overran(item: &str, runnable: &Runnable, waited: &Waited, stopping: &Option<String>) -> AxError {
    let seconds = TICK.saturating_mul(waited.patience).as_secs();
    let aftermath = match stopping {
        None => "it was stopped".to_owned(),
        Some(trouble) => trouble.clone(),
    };
    AxError::failure(
        AxCode::Timeout,
        "install a tool",
        format!(
            "{item}: {} was still running after {seconds}s, so {aftermath}",
            runnable.spelled()
        ),
    )
    .with_recovery(format!(
        "run this line yourself in a terminal, where it can take as long as it needs and ask \
         what it wants to ask; what it printed before it was stopped is in {}",
        waited.log.display()
    ))
}

/// The child could not be watched, which is not the same as failing.
fn unwatchable(
    item: &str,
    runnable: &Runnable,
    stopping: &Option<String>,
    err: &std::io::Error,
) -> AxError {
    let aftermath = match stopping {
        None => "it was stopped".to_owned(),
        Some(trouble) => trouble.clone(),
    };
    AxError::failure(
        AxCode::ToolOutcomeUnknown,
        "install a tool",
        format!(
            "{item}: {} could not be waited for: {err}; {aftermath}",
            runnable.spelled()
        ),
    )
    .with_recovery("check whether the tool is installed, and run the line yourself if it is not")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    use accounting::Recipe;

    /// A program that ends by itself, and one that ends in failure.
    fn ending(code: Code) -> Recipe {
        match (cfg!(target_os = "windows"), code) {
            (true, Code::Success) => Recipe::Command {
                program: "cmd",
                args: &["/C", "exit", "0"],
            },
            (true, Code::Failure) => Recipe::Command {
                program: "cmd",
                args: &["/C", "exit", "3"],
            },
            (false, Code::Success) => Recipe::Command {
                program: "sh",
                args: &["-c", "exit 0"],
            },
            (false, Code::Failure) => Recipe::Command {
                program: "sh",
                args: &["-c", "exit 3"],
            },
        }
    }

    /// A log file nobody reads, for the tests about waiting.
    fn scratch_log(test: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("sprawling-running-{test}.log"))
    }

    enum Code {
        Success,
        Failure,
    }

    /// A program that keeps running whatever its stdin is: it stands in
    /// for the package manager that is downloading, or wedged, when the
    /// knocks run out.
    fn never_ending() -> Recipe {
        if cfg!(target_os = "windows") {
            Recipe::Command {
                program: "ping",
                args: &["-n", "30", "127.0.0.1"],
            }
        } else {
            Recipe::Command {
                program: "sleep",
                args: &["30"],
            }
        }
    }

    /// A program that waits for a person to type something: it stands
    /// in for the source agreement and the password prompt.
    fn waiting_on_input() -> Recipe {
        if cfg!(target_os = "windows") {
            Recipe::Command {
                program: "cmd",
                args: &["/C", "pause"],
            }
        } else {
            Recipe::Command {
                program: "sh",
                args: &["-c", "read answer"],
            }
        }
    }

    #[test]
    fn a_program_that_never_ends_is_killed_when_the_knocks_run_out() {
        let recipe = never_ending();
        let runnable = recipe.command("pretend").unwrap();
        // Forty knocks at 50 ms: the child runs for thirty seconds, so
        // returning at all is the assertion, and the test costs two.
        let refused = run("pretend", &runnable, 40, &scratch_log("never_ends")).unwrap_err();
        assert_eq!(refused.code(), &AxCode::Timeout);
        assert!(
            refused.recovery().contains("run this line yourself"),
            "the refusal says what works instead: {}",
            refused.recovery()
        );
    }

    /// The stdin every child gets is end of input, so an installer that
    /// asks a question is answered at once rather than holding this
    /// thread for every knock it was given.
    #[test]
    fn a_program_that_asks_a_question_ends_long_before_the_knocks_do() {
        let recipe = waiting_on_input();
        let runnable = recipe.command("pretend").unwrap();
        // A hundred knocks is five seconds of patience; reading end of
        // input takes one. A `Timeout` here means stdin was not null.
        let outcome = run("pretend", &runnable, 100, &scratch_log("asks"));
        assert!(
            outcome.as_ref().err().map(AxError::code) != Some(&AxCode::Timeout),
            "it read end of input rather than waiting for a person: {outcome:?}"
        );
    }

    #[test]
    fn a_program_that_succeeds_is_reported_as_done() {
        let recipe = ending(Code::Success);
        assert!(
            run(
                "pretend",
                &recipe.command("pretend").unwrap(),
                PATIENCE,
                &scratch_log("succeeds")
            )
            .is_ok()
        );
    }

    #[test]
    fn a_program_that_fails_is_reported_with_the_line_a_person_can_rerun() {
        let recipe = ending(Code::Failure);
        let refused = run(
            "pretend",
            &recipe.command("pretend").unwrap(),
            PATIENCE,
            &scratch_log("fails"),
        )
        .unwrap_err();
        assert_eq!(refused.code(), &AxCode::ToolUnavailable);
        assert!(refused.to_string().contains("ended in failure"));
    }

    #[test]
    fn a_program_that_is_not_on_this_machine_is_refused_before_any_wait() {
        let recipe = Recipe::Command {
            program: "sprawling-no-such-installer",
            args: &[],
        };
        let runnable = recipe.command("pretend").unwrap();
        let refused = run("pretend", &runnable, PATIENCE, &scratch_log("absent")).unwrap_err();
        assert_eq!(refused.code(), &AxCode::ToolUnavailable);
    }

    /// What the installer printed is what a person reads when it fails,
    /// so it lands in the item's log rather than nowhere.
    #[test]
    fn what_an_installer_prints_lands_in_its_log() {
        let recipe = if cfg!(target_os = "windows") {
            Recipe::Command {
                program: "cmd",
                args: &["/C", "echo fetched & echo refused 1>&2 & exit 3"],
            }
        } else {
            Recipe::Command {
                program: "sh",
                args: &["-c", "echo fetched; echo refused >&2; exit 3"],
            }
        };
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("pretend.log");
        let refused = run(
            "pretend",
            &recipe.command("pretend").unwrap(),
            PATIENCE,
            &log,
        )
        .unwrap_err();
        let written = std::fs::read_to_string(&log).unwrap();
        assert_eq!(
            (
                written.contains("fetched"),
                written.contains("refused"),
                refused.recovery().contains(&log.display().to_string())
            ),
            (true, true, true),
            "the log holds both streams and the refusal names it: {written:?} / {}",
            refused.recovery()
        );
    }

    /// A compile from source gets the patience a real build needs, and
    /// a download keeps the three-minute ceiling.
    #[test]
    fn a_build_is_given_twenty_minutes_and_a_download_three() {
        let cargo = Recipe::Command {
            program: "cargo",
            args: &["install", "cargo-deny", "--locked"],
        };
        let winget = Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Casey.Just"],
        };
        let waits = [cargo, winget]
            .map(|recipe| TICK.saturating_mul(patience_for(&recipe.command("pretend").unwrap())));
        assert_eq!(
            waits,
            [Duration::from_secs(20 * 60), Duration::from_secs(180)]
        );
    }
}
