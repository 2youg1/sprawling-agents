// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one place this binary starts an install program, under a
//! patience it cannot outlive (sprawling-SPEC.md section 8-64).
//!
//! **A wait here is always bounded, and a child here can never wait on
//! a person.** The three facts that make that true are in `run` and
//! nowhere else: stdin, stdout and stderr are `Stdio::null()`, so a
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
//! `Runnable` is the recipe that passed the "may this city run it"
//! question, so nothing below re-asks it: an unrunnable recipe cannot
//! be spelled as this type (`Recipe::command`).

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use kernel::{AxCode, AxError};

/// How many times one install program is asked whether it has
/// finished before this city stops it.
///
/// `PATIENCE * TICK` is three minutes: long enough for a package
/// manager to download a toolchain over a slow link, and the ceiling
/// this project puts on any single wait, because a writer thread held
/// longer than that is indistinguishable from a hung city.
pub(crate) const PATIENCE: u32 = 3_600;

/// How long this city waits between two knocks. Short enough that a
/// fast install is not padded, long enough that three minutes of
/// waiting costs a few thousand cheap syscalls rather than a spinning
/// CPU.
const TICK: Duration = Duration::from_millis(50);

/// A program and its arguments that this city is allowed to start.
///
/// Constructed only by `Recipe::command`, which is where a recipe this
/// city may not run is refused, so holding one of these is the proof
/// that the question was asked and answered.
pub(crate) struct Runnable<'a> {
    program: &'a str,
    args: &'a [&'a str],
}

impl<'a> Runnable<'a> {
    pub(crate) fn new(program: &'a str, args: &'a [&'a str]) -> Runnable<'a> {
        Runnable { program, args }
    }

    /// The command as a person would type it into their own terminal.
    /// Every message about this program quotes this, so what a person
    /// is told to run is what this city ran.
    pub(crate) fn spelled(&self) -> String {
        if self.args.is_empty() {
            return self.program.to_owned();
        }
        format!("{} {}", self.program, self.args.join(" "))
    }
}

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
pub(crate) fn run(item: &str, runnable: &Runnable, patience: u32) -> Result<(), AxError> {
    let mut child = Command::new(runnable.program)
        .args(runnable.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "install a tool",
                format!("{item}: {}: {err}", runnable.program),
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
                .with_recovery("run this line yourself in a terminal to see what it reported"));
            }
            Ok(None) => {}
            Err(err) => {
                return Err(unwatchable(item, runnable, &stop(&mut child), &err));
            }
        }
        let Some(left) = knocks.checked_sub(1) else {
            return Err(overran(item, runnable, patience, &stop(&mut child)));
        };
        knocks = left;
        std::thread::sleep(TICK);
    }
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

/// The knocks ran out with the program still running.
fn overran(item: &str, runnable: &Runnable, patience: u32, stopping: &Option<String>) -> AxError {
    let seconds = TICK.saturating_mul(patience).as_secs();
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
    .with_recovery(
        "run this line yourself in a terminal: an installer that asks a question gets no answer \
         from this city, because it is started without a terminal to ask on",
    )
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

    /// A program that ends by itself, and one that ends in failure.
    fn ending(code: &'static str) -> (&'static str, Vec<&'static str>) {
        if cfg!(target_os = "windows") {
            ("cmd", vec!["/C", "exit", code])
        } else {
            ("sh", vec!["-c", "exit $0", code])
        }
    }

    /// A program that keeps running whatever its stdin is: it stands in
    /// for the package manager that is downloading, or wedged, when the
    /// knocks run out.
    fn never_ending() -> (&'static str, Vec<&'static str>) {
        if cfg!(target_os = "windows") {
            ("ping", vec!["-n", "30", "127.0.0.1"])
        } else {
            ("sleep", vec!["30"])
        }
    }

    /// A program that waits for a person to type something: it stands
    /// in for the source agreement and the password prompt.
    fn waiting_on_input() -> (&'static str, Vec<&'static str>) {
        if cfg!(target_os = "windows") {
            ("cmd", vec!["/C", "pause"])
        } else {
            ("sh", vec!["-c", "read answer"])
        }
    }

    #[test]
    fn a_program_that_never_ends_is_killed_when_the_knocks_run_out() {
        let (program, args) = never_ending();
        let runnable = Runnable::new(program, &args);
        // Forty knocks at 50 ms: the child runs for thirty seconds, so
        // returning at all is the assertion, and the test costs two.
        let refused = run("pretend", &runnable, 40).unwrap_err();
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
        let (program, args) = waiting_on_input();
        let runnable = Runnable::new(program, &args);
        // A hundred knocks is five seconds of patience; reading end of
        // input takes one. A `Timeout` here means stdin was not null.
        let outcome = run("pretend", &runnable, 100);
        assert!(
            outcome.as_ref().err().map(AxError::code) != Some(&AxCode::Timeout),
            "it read end of input rather than waiting for a person: {outcome:?}"
        );
    }

    #[test]
    fn a_program_that_succeeds_is_reported_as_done() {
        let (program, args) = ending("0");
        assert!(run("pretend", &Runnable::new(program, &args), PATIENCE).is_ok());
    }

    #[test]
    fn a_program_that_fails_is_reported_with_the_line_a_person_can_rerun() {
        let (program, args) = ending("3");
        let refused = run("pretend", &Runnable::new(program, &args), PATIENCE).unwrap_err();
        assert_eq!(refused.code(), &AxCode::ToolUnavailable);
        assert!(refused.to_string().contains("ended in failure"));
    }

    #[test]
    fn a_program_that_is_not_on_this_machine_is_refused_before_any_wait() {
        let runnable = Runnable::new("sprawling-no-such-installer", &[]);
        let refused = run("pretend", &runnable, PATIENCE).unwrap_err();
        assert_eq!(refused.code(), &AxCode::ToolUnavailable);
    }

    #[test]
    fn the_wait_this_city_installs_under_stays_inside_the_ceiling() {
        let bound = TICK.saturating_mul(PATIENCE);
        assert!(
            bound <= Duration::from_secs(180),
            "a single wait may not exceed three minutes: {bound:?}"
        );
    }
}
