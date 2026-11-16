// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::refusal::{Refusal, RefusalCode};
use crate::scope::Pattern;

const NAMED_IN_A_REFUSAL: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Named {
    pub(crate) title: String,
    pub(crate) process: String,
}

pub(crate) fn choose(
    seen: &[Named],
    title: Option<&str>,
    process: Option<&str>,
) -> Result<usize, Refusal> {
    let (None, None) = (title, process) else {
        return narrow(seen, title, process);
    };
    Err(Refusal::new(
        RefusalCode::InvalidArgs,
        "use the desktop",
        "the call named no window".to_owned(),
        "send `title` or `process`; `desktop.windows` reports both for everything this scope \
         lists",
    ))
}

fn narrow(seen: &[Named], title: Option<&str>, process: Option<&str>) -> Result<usize, Refusal> {
    let title = title.map(Pattern::new);
    let process = process.map(Pattern::new);
    let hits: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter(|(_at, window)| {
            title
                .as_ref()
                .is_none_or(|glob| glob.matches(&window.title))
                && process
                    .as_ref()
                    .is_none_or(|glob| glob.matches(&window.process))
        })
        .map(|(at, _window)| at)
        .collect();
    match hits.as_slice() {
        [only] => Ok(*only),
        [] => Err(Refusal::new(
            RefusalCode::InvalidArgs,
            "use the desktop",
            "no window on this desktop answers to that name".to_owned(),
            "call `desktop.windows` to see what is open; a window this scope does not list is \
             not reported there either",
        )),
        several => Err(Refusal::new(
            RefusalCode::InvalidArgs,
            "use the desktop",
            format!(
                "{} windows answer to that name: {}",
                several.len(),
                listed(seen, several)
            ),
            "name one of them exactly; acting on whichever one matched first would be a guess \
             at which document you meant",
        )),
    }
}

fn listed(seen: &[Named], hits: &[usize]) -> String {
    let mut named: Vec<String> = hits
        .iter()
        .take(NAMED_IN_A_REFUSAL)
        .filter_map(|at| seen.get(*at))
        .map(|window| format!("`{}`", window.title))
        .collect();
    if hits.len() > NAMED_IN_A_REFUSAL {
        named.push("...".to_owned());
    }
    named.join(", ")
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

    fn desktop() -> Vec<Named> {
        vec![
            Named {
                title: "a.txt — Notepad".to_owned(),
                process: "notepad.exe".to_owned(),
            },
            Named {
                title: "b.txt — Notepad".to_owned(),
                process: "notepad.exe".to_owned(),
            },
            Named {
                title: "Calculator".to_owned(),
                process: "ApplicationFrameHost.exe".to_owned(),
            },
        ]
    }

    #[test]
    fn one_match_is_the_answer_and_the_position_is_what_comes_back() {
        assert_eq!(choose(&desktop(), Some("Calculator"), None).unwrap(), 2);
        assert_eq!(choose(&desktop(), Some("a.txt*"), None).unwrap(), 0);

        assert_eq!(
            choose(&desktop(), Some("*Notepad*"), Some("notepad.exe"))
                .unwrap_err()
                .as_error()["data"]["code"],
            "E_INVALID_ARGS"
        );
        assert_eq!(
            choose(&desktop(), Some("b.txt*"), Some("notepad.exe")).unwrap(),
            1
        );
        assert!(choose(&desktop(), Some("Calculator"), Some("notepad.exe")).is_err());
    }

    #[test]
    fn two_matches_are_refused_and_both_titles_are_offered() {
        let refusal =
            choose(&desktop(), Some("*Notepad*"), None).expect_err("two matches is not a choice");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_INVALID_ARGS");
        let subject = error["data"]["subject"].as_str().unwrap();
        assert!(subject.contains("a.txt — Notepad"), "{subject}");
        assert!(subject.contains("b.txt — Notepad"), "{subject}");
        assert!(subject.starts_with("2 windows"), "{subject}");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("name one of them exactly")
        );
    }

    #[test]
    fn a_refusal_that_could_list_a_hundred_titles_lists_twelve_and_says_so() {
        let many: Vec<Named> = (0..40)
            .map(|at| Named {
                title: format!("doc{at} — Notepad"),
                process: "notepad.exe".to_owned(),
            })
            .collect();
        let refusal = choose(&many, Some("*Notepad*"), None).unwrap_err();
        let subject = refusal.as_error()["data"]["subject"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(subject.starts_with("40 windows"), "{subject}");
        assert!(subject.ends_with("..."), "{subject}");
        assert_eq!(subject.matches("Notepad").count(), NAMED_IN_A_REFUSAL);
    }

    #[test]
    fn a_name_nothing_answers_to_points_at_the_tool_that_says_what_is_open() {
        let refusal = choose(&desktop(), Some("Password Manager"), None).unwrap_err();
        assert!(
            refusal.as_error()["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("desktop.windows")
        );

        assert!(choose(&[], Some("*"), None).is_err());
    }

    #[test]
    fn naming_nothing_at_all_is_refused_before_anything_is_matched() {
        let refusal = choose(&desktop(), None, None).unwrap_err();
        assert!(
            refusal.as_error()["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("named no window")
        );
    }
}
