// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The GitHub CLI on this machine asked which login one host is signed in
//! as: the reader the served city hands its views for a `GithubLogin`
//! query (wire-SPEC.md 8-67, accounting-SPEC.md 8-18-3).
//!
//! **It never waits on a person and never keeps a secret.** stdin is
//! empty and `GH_PROMPT_DISABLED` is set, so `gh` fails rather than asks;
//! the wait is a counted set of knocks, as `doctor::running`'s is, and a
//! `gh` still running when they run out is stopped through
//! `running::stop`. stderr is never read and stdout keeps one line, the
//! login, so nothing `gh` prints about its credentials reaches the city.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// How many times `gh` is asked whether it has finished: `PATIENCE *
/// TICK` is fifteen seconds, longer than one API round trip on a slow
/// link and short enough that a person still watching the card is told.
const PATIENCE: u32 = 300;

/// How long this city waits between two knocks.
const TICK: Duration = Duration::from_millis(50);

/// The exit code `gh` documents for a command that needs a login it does
/// not have.
const NOT_LOGGED_IN: i32 = 4;

/// The login `host` is signed in as, read through this machine's search
/// path.
pub(crate) fn login(host: &str) -> wire::GithubReading {
    login_through(super::host::find_program, host)
}

/// The same, with the search path a parameter.
fn login_through(find: fn(&str) -> Option<PathBuf>, host: &str) -> wire::GithubReading {
    match find("gh") {
        None => wire::GithubReading::NoCli,
        Some(gh) => reading(&ask(&gh, host)),
    }
}

/// How one `gh` call ended.
enum Ended {
    /// It exited; `code` is absent when a signal ended it.
    Exited { code: Option<i32>, stdout: String },
    /// It would not start.
    Unstarted,
    /// It did not answer in time, or could not be watched, and was
    /// stopped; `stopping` is what went wrong while stopping it.
    Unanswered { stopping: Option<String> },
}

/// What one ending says about the host.
///
/// Every ending but a login on the first line of a clean exit leaves
/// the card as the person left it, so the failures differ only in the
/// way on the page offers for each.
fn reading(ended: &Ended) -> wire::GithubReading {
    match ended {
        Ended::Exited {
            code: Some(0),
            stdout,
        } => match stdout.lines().next().map(str::trim) {
            Some(login) if is_login(login) => wire::GithubReading::Found {
                login: login.to_owned(),
            },
            Some(_) | None => wire::GithubReading::Failed { exit: Some(0) },
        },
        Ended::Exited {
            code: Some(NOT_LOGGED_IN),
            ..
        } => wire::GithubReading::NotLoggedIn,
        Ended::Exited { code, .. } => wire::GithubReading::Failed { exit: *code },
        Ended::Unstarted | Ended::Unanswered { stopping: None } => {
            wire::GithubReading::Failed { exit: None }
        }
        // A process this city started and could not stop is still holding
        // whatever it held, and the person is the one who can end it.
        Ended::Unanswered {
            stopping: Some(why),
        } => wire::GithubReading::Stuck { why: why.clone() },
    }
}

/// Whether a line is a GitHub login: letters, digits, `-` and the `_` an
/// enterprise's managed users carry, nothing else.
fn is_login(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// Starts `gh api --hostname <host> user --jq .login` and waits for it,
/// a counted number of knocks at most.
fn ask(gh: &Path, host: &str) -> Ended {
    let started = Command::new(gh)
        .args(["api", "--hostname", host, "user", "--jq", ".login"])
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = started else {
        return Ended::Unstarted;
    };
    let mut knocks = PATIENCE;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ended::Exited {
                    code: status.code(),
                    stdout: drained(&mut child),
                };
            }
            Ok(None) => {}
            Err(_unwatchable) => {
                return Ended::Unanswered {
                    stopping: super::running::stop(&mut child),
                };
            }
        }
        let Some(left) = knocks.checked_sub(1) else {
            return Ended::Unanswered {
                stopping: super::running::stop(&mut child),
            };
        };
        knocks = left;
        std::thread::sleep(TICK);
    }
}

/// What an exited `gh` wrote to stdout. A pipe that will not read gives
/// nothing, which reads as no login.
fn drained(child: &mut Child) -> String {
    let mut out = String::new();
    match child.stdout.take() {
        Some(mut stdout) => match stdout.read_to_string(&mut out) {
            Ok(_) => out,
            Err(_unreadable) => String::new(),
        },
        None => out,
    }
}

#[cfg(test)]
mod tests {
    use super::{Ended, login_through, reading};

    #[test]
    fn no_gh_on_the_search_path_is_no_cli() {
        assert_eq!(
            login_through(|_| None, "github.com"),
            wire::GithubReading::NoCli
        );
    }

    #[test]
    fn exit_four_is_not_logged_in() {
        assert_eq!(
            reading(&Ended::Exited {
                code: Some(4),
                stdout: String::new(),
            }),
            wire::GithubReading::NotLoggedIn
        );
    }

    #[test]
    fn a_login_on_the_first_line_is_found() {
        assert_eq!(
            reading(&Ended::Exited {
                code: Some(0),
                stdout: "octocat\n".to_owned(),
            }),
            wire::GithubReading::Found {
                login: "octocat".to_owned(),
            }
        );
    }
}
