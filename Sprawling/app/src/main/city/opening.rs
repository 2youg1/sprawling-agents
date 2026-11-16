// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Entrance {

    Bare,

    Up,

    Serve,
}

impl Entrance {

    pub(crate) fn opens(self) -> Open {
        match self {
            Entrance::Up => Open::Browser,
            Entrance::Bare | Entrance::Serve => Open::Nothing,
        }
    }

    pub(crate) fn surface(self, args: &[String], interactive: Interactive) -> Option<Surface> {
        if args.iter().any(|a| a == "--no-console") {
            return None;
        }
        let face = match self {
            Entrance::Bare => Surface::Cli,
            Entrance::Up => Surface::QuietHost,
            Entrance::Serve if args.iter().any(|a| a == "--console") => Surface::Cli,
            Entrance::Serve => return None,
        };
        Some(match interactive {
            Interactive::Terminal => face,
            Interactive::Pipe => Surface::Headless,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interactive {
    Terminal,
    Pipe,
}

impl Interactive {
    pub(crate) fn here() -> Interactive {
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            Interactive::Terminal
        } else {
            Interactive::Pipe
        }
    }
}

use sprawling::console::Surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Open {

    Browser,

    Nothing,
}

const OPEN_VARIABLE: &str = "SPRAWLING_OPEN";

const NEVER: &str = "never";

pub(super) fn opening(args: &[String], by_itself: Open) -> Open {
    let asked_for = std::env::var(OPEN_VARIABLE).ok();
    deciding(args, by_itself, asked_for.as_deref())
}

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

        assert!(matches!(
            deciding(&args(&[]), Open::Browser, None),
            Open::Browser
        ));
        assert!(matches!(
            deciding(&args(&[]), Open::Nothing, None),
            Open::Nothing
        ));

        for entrance in [Open::Browser, Open::Nothing] {
            assert!(matches!(
                deciding(&args(&["--no-open"]), entrance, None),
                Open::Nothing
            ));
            assert!(matches!(
                deciding(&args(&[]), entrance, Some("never")),
                Open::Nothing
            ));

            assert!(matches!(
                deciding(&args(&["--open", "--no-open"]), entrance, None),
                Open::Nothing
            ));

            assert!(matches!(
                deciding(&args(&["--open"]), entrance, None),
                Open::Browser
            ));
        }

        assert!(matches!(
            deciding(&args(&[]), Open::Browser, Some("always")),
            Open::Browser
        ));
    }
}
