// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which host a GitHub import asks, whether it names a host at all, and
//! the reader the served city handed in (accounting-SPEC.md 8-18-3,
//! wire-SPEC.md 8-67).
//!
//! Starting `gh` reaches this machine and the network, so the reader is
//! the binary's (`bin::doctor::github`) and arrives as a `fn` pointer;
//! what is judged here is the city's own reading of the question.

use super::super::prepared::unavailable;

/// Not built yet: every import is answered as a question this city
/// could not look up.
pub(crate) fn github_answer(
    _ask: Option<fn(&str) -> wire::GithubReading>,
    host: Option<&str>,
) -> wire::Answer {
    unavailable(format!("GithubLogin({host:?})"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::sync::Mutex;

    use super::super::super::holding::Views;

    /// Every host the scripted reader was asked, in order.
    static ASKED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    /// A reader that finds the same login on every host it is asked.
    fn scripted(host: &str) -> wire::GithubReading {
        ASKED.lock().unwrap().push(host.to_owned());
        wire::GithubReading::Found {
            login: "octocat".to_owned(),
        }
    }

    fn import(views: &Views, host: Option<&str>) -> wire::Answer {
        views
            .prepare(&wire::Query::GithubLogin {
                host: host.map(str::to_owned),
            })
            .finish()
    }

    fn answered(host: &str, reading: wire::GithubReading) -> wire::Answer {
        wire::Answer::GithubLogin(wire::GithubLoginAnswer {
            host: host.to_owned(),
            reading,
        })
    }

    /// The page that names no host asks `github.com`; a string `gh` would
    /// read as an option of its own never reaches it; views nobody served
    /// do not reach this machine at all.
    #[test]
    fn a_github_login_is_asked_of_the_reader_the_views_were_handed() {
        let dir = tempfile::tempdir().unwrap();
        let mut served = Views::new(dir.path());
        served.ask_github_through(scripted);
        let unserved = Views::new(dir.path());
        let found = || wire::GithubReading::Found {
            login: "octocat".to_owned(),
        };

        let answers = vec![
            import(&served, None),
            import(&served, Some("--hostname=elsewhere")),
            import(&served, Some("ghe.example.com:8443")),
            import(&unserved, None),
        ];

        assert_eq!(
            (answers, ASKED.lock().unwrap().clone()),
            (
                vec![
                    answered("github.com", found()),
                    answered("--hostname=elsewhere", wire::GithubReading::NotAHost),
                    answered("ghe.example.com:8443", found()),
                    wire::Answer::Unavailable {
                        query: "GithubLogin(github.com)".to_owned(),
                    },
                ],
                vec!["github.com".to_owned(), "ghe.example.com:8443".to_owned()],
            )
        );
    }
}
