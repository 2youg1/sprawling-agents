// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether this invocation opens the person's browser, and whether the
//! terminal becomes the city's console.
//!
//! **One rule, one evaluation point, two defaults.** `up` is the front
//! door, and showing a person their own city is what it is for, so it
//! opens unless refused. `serve` is what a script drives, so it stays
//! out of the way unless asked. Both read the refusal here, because a
//! second place deciding it is how one entrance comes to disbelieve the
//! instruction given to another.
//!
//! The refusal has two ways in only because its callers differ in kind:
//! a person at a terminal writes `--no-open`, and a process that
//! inherited a city from the first-run wizard sets
//! `SPRAWLING_OPEN=never` — the wizard calls back into the entrances
//! with no arguments of its own, because the person already answered by
//! pressing a key. A refusal beats a request, because whoever added one
//! was answering a command line they did not write.
//!
//! That the console keys off this same answer is deliberate rather than
//! incidental: a run told to leave the screen alone is not one to take
//! the screen over. `--console` restores it for the narrower case.

/// What serving does with the person's browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Open {
    /// Show the city: the appliance path, where the window is the point.
    Browser,
    /// Leave the screen alone, which is what a service wants.
    Nothing,
}

/// The variable that refuses the window from outside a command line.
///
/// A flag cannot reach the first-run wizard: it calls back into these
/// entrances with no arguments of its own, because the person already
/// answered by pressing a key. A supervisor that runs a city as a
/// service has its own argv, but the wizard it launched does not.
const OPEN_VARIABLE: &str = "SPRAWLING_OPEN";

/// What refusing the window says, either way in.
const NEVER: &str = "never";

pub(super) fn opening(args: &[String], by_itself: Open) -> Open {
    let asked_for = std::env::var(OPEN_VARIABLE).ok();
    deciding(args, by_itself, asked_for.as_deref())
}

/// The rule itself, given what the environment said.
///
/// Split from the read so the rule can be asserted without a test
/// setting a process-wide variable, which no test may do to another.
fn deciding(args: &[String], by_itself: Open, variable: Option<&str>) -> Open {
    let refused = args.iter().any(|a| a == "--no-open") || variable == Some(NEVER);
    if refused {
        return Open::Nothing;
    }
    if args.iter().any(|a| a == "--open") {
        return Open::Browser;
    }
    by_itself
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code"
    )]

    use super::{Open, deciding};

    fn args(said: &[&str]) -> Vec<String> {
        said.iter().map(|a| (*a).to_owned()).collect()
    }

    #[test]
    fn a_refusal_to_open_beats_a_request_and_the_entrances_keep_their_own_defaults() {
        // The two defaults, which are the whole reason `up` and `serve`
        // cannot share one: the front door exists to show a person their
        // city, the scripted path exists not to.
        assert!(matches!(
            deciding(&args(&[]), Open::Browser, None),
            Open::Browser
        ));
        assert!(matches!(
            deciding(&args(&[]), Open::Nothing, None),
            Open::Nothing
        ));
        // Either way of refusing reaches either entrance, which is what
        // an automated run against a clean root had no way to say.
        for entrance in [Open::Browser, Open::Nothing] {
            assert!(matches!(
                deciding(&args(&["--no-open"]), entrance, None),
                Open::Nothing
            ));
            assert!(matches!(
                deciding(&args(&[]), entrance, Some("never")),
                Open::Nothing
            ));
            // A refusal wins over a request: whoever added it was
            // answering a command line they did not write.
            assert!(matches!(
                deciding(&args(&["--open", "--no-open"]), entrance, None),
                Open::Nothing
            ));
            // A refusal only beats a request; it is not a default.
            assert!(matches!(
                deciding(&args(&["--open"]), entrance, None),
                Open::Browser
            ));
        }
        // Any other value says nothing, rather than meaning something.
        assert!(matches!(
            deciding(&args(&[]), Open::Browser, Some("always")),
            Open::Browser
        ));
    }
}
