// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::holding::Views;
use super::super::prepared::{Prepared, unavailable};

impl Views {

    pub(in crate::views) fn github_of(&self, host: Option<&str>) -> Prepared {
        Prepared::GithubLogin {
            ask: self.reach.github,
            host: host.map(str::to_owned),
        }
    }
}

pub(crate) const DEFAULT_HOST: &str = "github.com";

pub(crate) fn github_answer(
    ask: Option<fn(&str) -> wire::GithubReading>,
    host: Option<&str>,
) -> wire::Answer {
    let host = host.unwrap_or(DEFAULT_HOST);
    let Some(login) = ask else {
        return unavailable(format!("GithubLogin({host})"));
    };
    let reading = if is_host(host) {
        login(host)
    } else {
        wire::GithubReading::NotAHost
    };
    wire::Answer::GithubLogin(wire::GithubLoginAnswer {
        host: host.to_owned(),
        reading,
    })
}

fn is_host(host: &str) -> bool {
    !host.is_empty()
        && !host.starts_with('-')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::sync::Mutex;

    use super::Views;

    static ASKED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn scripted(host: &str) -> wire::GithubReading {
        ASKED.lock().unwrap().push(host.to_owned());
        wire::GithubReading::Found {
            login: "octocat".to_owned(),
        }
    }

    fn import(views: &Views, host: Option<&str>) -> wire::Answer {
        views
            .prepare(&wire::Query::GithubLogin(host.map(str::to_owned)))
            .finish()
    }

    fn answered(host: &str, reading: wire::GithubReading) -> wire::Answer {
        wire::Answer::GithubLogin(wire::GithubLoginAnswer {
            host: host.to_owned(),
            reading,
        })
    }

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
                        reason: None,
                    },
                ],
                vec!["github.com".to_owned(), "ghe.example.com:8443".to_owned()],
            )
        );
    }
}
