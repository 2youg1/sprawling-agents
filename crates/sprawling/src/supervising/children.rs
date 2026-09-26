// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The processes a supervised city runs as: `serve` in a child, and
//! `resume` in a child after each crash (sprawling-SPEC.md section 8-90).
//!
//! Thin on purpose. The exit a child leaves is read by `Closing::of`
//! and judged by `CrashBudget::after`; this file only spawns, waits,
//! samples the clock, and prints what the person needs to see.

use super::{CrashBudget, Next};
use crate::assembly::{Closing, now_ms};
use kernel::{AxCode, AxError};
use std::path::{Path, PathBuf};
use std::process::Command;

/// How a supervised city stopped being supervised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// The person closed the city.
    Chosen,
    /// The crash budget ran out and nobody lifted it.
    Degraded,
}

/// Which child `serve` this is, so only the first may open the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Launch {
    First,
    Again,
}

/// Serves `city` at `addr` in a child process and raises it again after
/// each crash until the person closes it or the crash budget runs out.
///
/// `line` is the command line `up` or `serve` received; `--supervise` is
/// taken out and the rest reaches every child `serve` unchanged.
///
/// # Errors
/// Returns the failure to find this binary, to start a child, or to read
/// the clock; a child that fails is a crash, not an error.
pub fn supervise(city: &Path, addr: &str, line: &[String]) -> Result<Ended, AxError> {
    let exe = std::env::current_exe().map_err(|err| {
        AxError::failure(AxCode::PathNotFound, "find this binary", err.to_string())
            .with_recovery("run the binary by its path rather than through a shim")
    })?;
    let mut budget = CrashBudget::fresh();
    let mut launch = Launch::First;
    loop {
        let served = run(&exe, &serve_line(city, addr, line, launch))?;
        launch = Launch::Again;
        match budget.after(&Closing::of(&served), now_ms()?) {
            Next::Stop => return Ok(Ended::Chosen),
            Next::Restart(kept) => {
                budget = kept;
                eprintln!("supervise: the city crashed; resuming it and serving it again");
            }
            Next::Degraded { crashes, cause } => {
                eprintln!("supervise: {crashes} crashes inside a minute; the last one: {cause}");
                eprintln!(
                    "supervise: the city stays down. Press Enter to try again, or close this terminal."
                );
                if !lifted() {
                    return Ok(Ended::Degraded);
                }
                budget = CrashBudget::fresh();
            }
        }
        // A resume that fails is printed by the child itself; the serve
        // after it meets the same fault and spends the budget.
        if let Err(failure) = run(&exe, &resume_line(city))? {
            eprintln!("supervise: resume did not finish ({failure}); serving anyway");
        }
    }
}

/// Whether the person pressed Enter; end of input means nobody is there.
fn lifted() -> bool {
    let mut answer = String::new();
    matches!(std::io::stdin().read_line(&mut answer), Ok(read) if read > 0)
}

/// Runs one child to its end and reads its exit as served or failed.
fn run(exe: &PathBuf, args: &[String]) -> Result<Result<(), AxError>, AxError> {
    let status = Command::new(exe).args(args).status().map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "start a supervised child",
            format!("{}: {err}", exe.display()),
        )
        .with_recovery("check that this binary is still where it was started from")
    })?;
    Ok(if status.success() {
        Ok(())
    } else {
        Err(AxError::failure(
            AxCode::StorageFatal,
            "serve the city in a child process",
            status.to_string(),
        )
        .with_recovery("read the child's own output above this line"))
    })
}

fn serve_line(city: &Path, addr: &str, line: &[String], launch: Launch) -> Vec<String> {
    let wants_up = line.first().is_some_and(|verb| verb == "up");
    let opens = match launch {
        Launch::First
            if !line.iter().any(|a| a == "--no-open")
                && (wants_up || line.iter().any(|a| a == "--open")) =>
        {
            "--open"
        }
        Launch::First | Launch::Again => "--no-open",
    };
    let console = wants_up && !line.iter().any(|a| a == "--no-console");
    [
        "serve".to_owned(),
        city.display().to_string(),
        addr.to_owned(),
    ]
    .into_iter()
    .chain(flags(line).filter(|a| a != "--open" && a != "--no-open"))
    .chain(std::iter::once(opens.to_owned()))
    .chain(console.then(|| "--console".to_owned()))
    .collect()
}

/// The flags of `line`, without its verb, its positionals, or `--supervise`.
fn flags(line: &[String]) -> impl Iterator<Item = String> + '_ {
    let mut takes_value = false;
    line.iter()
        .filter(move |word| {
            let keep = takes_value || word.starts_with("--");
            takes_value = matches!(word.as_str(), "--log" | "--web-dir");
            keep
        })
        .filter(|word| *word != "--supervise")
        .cloned()
}

fn resume_line(city: &Path) -> Vec<String> {
    vec!["resume".to_owned(), city.display().to_string()]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn a_child_of_up_no_open_does_not_enter_the_console() {
        let line: Vec<String> = ["up", "c", "--no-open", "--supervise"]
            .map(str::to_owned)
            .to_vec();
        let child = serve_line(Path::new("c"), "127.0.0.1:1", &line, Launch::First);
        assert!(!child.iter().any(|word| word == "--console"), "{child:?}");
    }
}
