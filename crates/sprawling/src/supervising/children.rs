// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The processes a supervised city runs as: `serve` in a child, and
//! `resume` in a child after each crash (`crates/sprawling/spec/Supervising.lean` §8-109).
//!
//! Thin on purpose. The exit a child leaves is read as served or failed
//! and judged by `CrashBudget::after`; this file only spawns, waits,
//! measures elapsed time, and prints what the person needs to see.

use super::{CrashBudget, Next};
use crate::serving::standing::monotonic_now;
use kernel::{AxCode, AxError, TimeMs};
use std::path::Path;
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

/// What `serve_city` decided for the child `serve`; this file only turns
/// the decisions into flags, so neither is worked out a second time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Child {
    /// The person's flags with their values, less `--supervise` and the
    /// four flags `first` and `console` stand for.
    pub forwarded: Vec<String>,
    /// Whether the first child opens the browser; every later one leaves it.
    pub first: Window,
    /// Whether every child enters the console.
    pub console: Console,
}

/// Whether a child `serve` opens the person's browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Open,
    Leave,
}

/// Whether a child `serve` enters the city's console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Console {
    Enter,
    Skip,
}

/// Serves `city` at `addr` in a child process and raises it again after
/// each crash until the person closes it or the crash budget runs out.
///
/// `child` carries what the unsupervised run would have decided.
///
/// # Errors
/// Returns the failure to find this binary or to start a child; a child
/// that fails is a crash, not an error.
pub fn supervise(city: &Path, addr: &str, child: &Child) -> Result<Ended, AxError> {
    let exe = std::env::current_exe().map_err(|err| {
        AxError::failure(AxCode::PathNotFound, "find this binary", err.to_string())
            .with_recovery("run the binary by its path rather than through a shim")
    })?;
    // Crashes are counted against elapsed time, not the wall clock: a
    // clock the system steps back would keep an old crash in the window,
    // and one stepped forward would drop a recent one.
    let supervised = monotonic_now();
    let mut budget = CrashBudget::fresh();
    let mut launch = Launch::First;
    loop {
        let served = run(&exe, &serve_line(city, addr, child, launch))?;
        launch = Launch::Again;
        let since = monotonic_now().saturating_duration_since(supervised);
        let at = TimeMs::new(u64::try_from(since.as_millis()).unwrap_or(u64::MAX));
        match budget.after(&served, at) {
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

/// Whether the person pressed Enter; end of input means nobody is there,
/// and a terminal that cannot be read is said out loud before the city
/// stays down.
fn lifted() -> bool {
    let mut answer = String::new();
    match std::io::stdin().read_line(&mut answer) {
        Ok(read) => read > 0,
        Err(err) => {
            eprintln!("supervise: the terminal could not be read ({err}); the city stays down");
            false
        }
    }
}

/// Runs one child to its end and reads its exit as served or failed.
#[expect(
    clippy::disallowed_methods,
    reason = "the supervising parent's child is the city itself and keeps this console on purpose (child §3)"
)]
fn run(exe: &Path, args: &[String]) -> Result<Result<(), AxError>, AxError> {
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

fn serve_line(city: &Path, addr: &str, child: &Child, launch: Launch) -> Vec<String> {
    let opens = match (launch, child.first) {
        (Launch::First, Window::Open) => "--open",
        (Launch::First, Window::Leave) | (Launch::Again, Window::Open | Window::Leave) => {
            "--no-open"
        }
    };
    let console = match child.console {
        Console::Enter => "--console",
        Console::Skip => "--no-console",
    };
    [
        "serve".to_owned(),
        city.display().to_string(),
        addr.to_owned(),
    ]
    .into_iter()
    .chain(child.forwarded.iter().cloned())
    .chain([opens.to_owned(), console.to_owned()])
    .collect()
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
        let child = Child {
            forwarded: vec!["--log".to_owned(), "debug".to_owned()],
            first: Window::Leave,
            console: Console::Skip,
        };
        let line = serve_line(Path::new("c"), "127.0.0.1:1", &child, Launch::First);
        assert_eq!(
            line,
            [
                "serve",
                "c",
                "127.0.0.1:1",
                "--log",
                "debug",
                "--no-open",
                "--no-console"
            ]
            .map(str::to_owned)
            .to_vec()
        );
        let opened = Child {
            first: Window::Open,
            console: Console::Enter,
            ..child
        };
        let again = serve_line(Path::new("c"), "127.0.0.1:1", &opened, Launch::Again);
        assert_eq!(again[5..], ["--no-open".to_owned(), "--console".to_owned()]);
    }
}
