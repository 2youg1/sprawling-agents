// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How long a caller waits for a child, and how that child's ending is
//! read.
//!
//! Both facts decide what a caller writes down. The window decides
//! whether a command comes back as `Settled` or as `Backgrounded`, and
//! those two answers carry different payloads; the ending decides what
//! the `exec` result says happened. Neither may be spelled twice, so
//! both live here (runtime-SPEC 8-28-1).

use std::process::ExitStatus;
use std::time::Duration;

/// The short window a caller blocks for, counted in polls.
///
/// Injected rather than read from a constant where the waiting happens.
/// A caller that cannot choose the window has to spend the production
/// one to observe either answer, so the test that proves a command is
/// handed to the background costs ten seconds - and a suite that cannot
/// afford that instead proves nothing about the shape it ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollBudget {
    polls: u32,
    interval_ms: u64,
}

impl PollBudget {
    /// What a person gets: 500 polls of 20 ms, which is ten seconds.
    pub const DEFAULT: PollBudget = PollBudget {
        polls: 500,
        interval_ms: 20,
    };

    /// A window of this many polls, waiting this long between them.
    #[must_use]
    pub fn new(polls: u32, interval_ms: u64) -> PollBudget {
        PollBudget { polls, interval_ms }
    }

    pub(super) fn polls(self) -> u32 {
        self.polls
    }

    pub(super) fn interval(self) -> Duration {
        Duration::from_millis(self.interval_ms)
    }
}

impl Default for PollBudget {
    fn default() -> PollBudget {
        PollBudget::DEFAULT
    }
}

/// How a child process stopped.
///
/// Three answers rather than one number. `-1` used to mean "it returned
/// minus one", "a signal stopped it" and "this city never found out",
/// and the reader of a tool result could not tell which of the three had
/// happened - while a program that really does return minus one is
/// ordinary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// It ran to the end and returned this code.
    Ended { code: i32 },
    /// A signal stopped it, so there is no code to return. This is what
    /// `halt` leaves behind on a Unix host.
    Signalled,
    /// This city cannot say how it stopped.
    Unknown { why: Unseen },
}

impl Exit {
    /// Reads an ending off the status the operating system reported.
    pub(super) fn of(status: &ExitStatus) -> Exit {
        match status.code() {
            Some(code) => Exit::Ended { code },
            None => Exit::Signalled,
        }
    }

    /// What this ending is called wherever it is written down.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Exit::Ended { .. } => "ended",
            Exit::Signalled => "signalled",
            Exit::Unknown { .. } => "unknown",
        }
    }
}

/// Why this city cannot say how a child stopped.
///
/// Two situations, because the caller is owed different things by them:
/// a member that left the table was never waited on at all, while a wait
/// that the host refused happened and gave no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unseen {
    /// The member was gone from the table before it could be waited on.
    LeftTheTable,
    /// The host refused to report the child's state.
    WaitRefused,
}

impl Unseen {
    /// The sentence a reader of a tool result is given.
    #[must_use]
    pub fn sentence(self) -> &'static str {
        match self {
            Unseen::LeftTheTable => {
                "this command left the table before anything waited on it, so its \
                 ending was never read"
            }
            Unseen::WaitRefused => {
                "this machine refused to report the child's state, so its ending is \
                 not known"
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    /// The three endings are three spellings, and the one that carries a
    /// number is the only one that carries a number.
    #[test]
    fn every_ending_says_which_of_the_three_it_is() {
        assert_eq!(Exit::Ended { code: -1 }.as_str(), "ended");
        assert_eq!(Exit::Signalled.as_str(), "signalled");
        assert_eq!(
            Exit::Unknown {
                why: Unseen::WaitRefused
            }
            .as_str(),
            "unknown"
        );
        assert_ne!(
            Exit::Ended { code: -1 },
            Exit::Signalled,
            "a program that returns minus one has not been signalled"
        );
    }

    /// The window a person gets is the one the SPEC states.
    #[test]
    fn the_default_window_is_ten_seconds_of_short_polls() {
        let budget = PollBudget::DEFAULT;
        assert_eq!(budget.polls(), 500);
        assert_eq!(budget.interval(), Duration::from_millis(20));
    }
}
