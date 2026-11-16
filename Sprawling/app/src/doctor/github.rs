// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::asking::Ended;

const PATIENCE: u32 = 300;

const NOT_LOGGED_IN: i32 = 4;

pub(crate) fn login(host: &str) -> wire::GithubReading {
    login_through(super::host::find_program, host)
}

fn login_through(find: fn(&str) -> Option<PathBuf>, host: &str) -> wire::GithubReading {
    match find("gh") {
        None => wire::GithubReading::NoCli,
        Some(gh) => reading(&ask(&gh, host)),
    }
}

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

        Ended::Unanswered {
            stopping: Some(why),
        } => wire::GithubReading::Stuck { why: why.clone() },
    }
}

fn is_login(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

fn first_line() -> impl FnMut(&str) -> bool + Send + 'static {
    let mut offered = false;
    move |_line| !std::mem::replace(&mut offered, true)
}

fn ask(gh: &Path, host: &str) -> Ended {
    super::asking::ask(
        child::command(gh)
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
