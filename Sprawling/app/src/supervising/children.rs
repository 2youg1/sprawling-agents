// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{CrashBudget, Next};
use crate::serving::standing::monotonic_now;
use kernel::{AxCode, AxError, TimeMs};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {

    Chosen,

    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Launch {
    First,
    Again,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Child {

    pub forwarded: Vec<String>,

    pub first: Window,

    pub console: Console,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Open,
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Console {
    Enter,
    Skip,
}

pub fn supervise(city: &Path, addr: &str, child: &Child) -> Result<Ended, AxError> {
    let exe = std::env::current_exe().map_err(|err| {
        AxError::failure(AxCode::PathNotFound, "find this binary", err.to_string())
            .with_recovery("run the binary by its path rather than through a shim")
    })?;

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

        if let Err(failure) = run(&exe, &resume_line(city))? {
            eprintln!("supervise: resume did not finish ({failure}); serving anyway");
        }
    }
}

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
