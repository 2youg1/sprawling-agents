// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The GitHub CLI on this machine asked which login one host is signed in
//! as: the reader the served city hands its views for a `GithubLogin`
//! query (`crates/wire/Spec.lean` §8-67, `crates/accounting/Spec.lean` §8-18-3).
//!
//! **It never waits on a person and never keeps a secret.** stdin is
//! empty and `GH_PROMPT_DISABLED` is set, so `gh` fails rather than asks;
//! the wait is `doctor::asking`'s counted one, and a `gh` still running
//! when it ends is stopped there. stderr is never read and stdout keeps
//! one line, the login, so nothing `gh` prints about its credentials
//! reaches the city.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::asking::Ended;

/// How many times `gh` is asked whether it has finished: `PATIENCE *
/// asking::TICK` is fifteen seconds, longer than one API round trip on a
/// slow link and short enough that a person still watching the card is
/// told.
const PATIENCE: u32 = 300;

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

/// What one ending says about the host.
///
/// Every ending but a login on the first line of a clean exit leaves
/// the card as the person left it, so the failures differ only in the
/// way on the page offers for each.
fn reading(ended: &Ended) -> wire::GithubReading {
    match ended {
        Ended::Exited {
            code: Some(0),
            kept,
        } => match kept.lines().next().map(str::trim) {
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

/// A keeper that chooses the first line `gh` prints and no other: the
/// login is that line, and nothing after it is read into this process.
fn first_line() -> impl FnMut(&str) -> bool + Send + 'static {
    let mut offered = false;
    move |_line| !std::mem::replace(&mut offered, true)
}

/// Starts `gh api --hostname <host> user --jq .login` and waits for it.
fn ask(gh: &Path, host: &str) -> Ended {
    super::asking::ask(
        Command::new(gh)
            .args(["api", "--hostname", host, "user", "--jq", ".login"])
            .env("GH_PROMPT_DISABLED", "1"),
        PATIENCE,
        first_line(),
    )
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
                kept: String::new(),
            }),
            wire::GithubReading::NotLoggedIn
        );
    }

    #[test]
    fn a_login_on_the_first_line_is_found() {
        assert_eq!(
            reading(&Ended::Exited {
                code: Some(0),
                kept: "octocat\n".to_owned(),
            }),
            wire::GithubReading::Found {
                login: "octocat".to_owned(),
            }
        );
    }
}
