// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::process::ExitStatus;
use std::time::Duration;

use kernel::AxError;

use super::ceiling::Ceiling;
use super::tail::Tail;
use super::{BacklogId, Body, Claim, Member, Table};

const READ_BYTES_PER_MS: u64 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollBudget {
    polls: u32,
    interval_ms: u64,
}

impl PollBudget {
    pub const DEFAULT: PollBudget = PollBudget {
        polls: 500,
        interval_ms: 20,
    };

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

    pub(super) fn read_per_poll(self) -> usize {
        usize::try_from(self.interval_ms.saturating_mul(READ_BYTES_PER_MS)).unwrap_or(usize::MAX)
    }
}

impl Default for PollBudget {
    fn default() -> PollBudget {
        PollBudget::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Ended { code: i32 },

    Signalled,

    Unknown { why: Unseen },
}

impl Exit {
    pub(super) fn of(status: &ExitStatus) -> Exit {
        match status.code() {
            Some(code) => Exit::Ended { code },
            None => Exit::Signalled,
        }
    }

    pub(super) fn polled(child: &mut std::process::Child) -> Option<Exit> {
        match child.try_wait() {
            Ok(Some(status)) => Some(Exit::of(&status)),
            Ok(None) => None,
            Err(_) => Some(Exit::Unknown {
                why: Unseen::WaitRefused,
            }),
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Exit::Ended { .. } => "ended",
            Exit::Signalled => "signalled",
            Exit::Unknown { .. } => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unseen {
    LeftTheTable,

    WaitRefused,
}

impl Unseen {
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

impl super::Backlog {
    pub(super) fn settle(&self, id: BacklogId) -> Result<Option<(Exit, Option<Ceiling>)>, AxError> {
        let mut table = self.hold()?;
        let Table { members, jobs, .. } = &mut *table;
        let Some(Member {
            body:
                Body::Command {
                    child,
                    claim,
                    ceiling,
                    ..
                },
            ..
        }) = members.get_mut(&id)
        else {
            return Ok(Some((
                Exit::Unknown {
                    why: Unseen::LeftTheTable,
                },
                None,
            )));
        };
        let stopped = match child.poll() {
            Ok(stopped) => stopped,
            Err(err) => {
                if let Claim::Window(owner) = *claim {
                    *claim = Claim::Run(owner);
                }
                return Err(err);
            }
        };
        let Some(exit) = stopped else {
            return Ok(None);
        };
        let reported = match *claim {
            Claim::Window(owner) | Claim::Run(owner) => jobs.ceiling(owner, *ceiling),
            Claim::Nobody => None,
        };
        members.remove(&id);
        Ok(Some((exit, reported)))
    }

    pub(super) fn hand_over(&self, id: BacklogId, read: Tail) -> Result<(), AxError> {
        let mut table = self.hold()?;
        if let Some(Member {
            body: Body::Command { claim, tail, .. },
            ..
        }) = table.members.get_mut(&id)
            && let Claim::Window(owner) = *claim
        {
            *claim = Claim::Run(owner);
            *tail = read;
        }
        Ok(())
    }
}

impl super::Backlog {
    pub(super) fn watch(
        &self,
        id: BacklogId,
        owner: kernel::RunId,
        what: String,
        dir: std::path::PathBuf,
    ) -> Result<super::Started, AxError> {
        let mut tail = Tail::default();
        for _ in 0..self.window.polls() {
            if let Some((exit, ceiling)) = self.settle(id)? {
                let (stdout, stderr) = super::collect(&dir);
                return Ok(super::Started::Settled {
                    exit,
                    stdout,
                    stderr,
                    ceiling,
                });
            }
            if let Some(sink) = &self.sink {
                sink.deliver(tail.take(&dir, (owner, id), self.window.read_per_poll()));
            }
            std::thread::sleep(self.window.interval());
        }
        self.hand_over(id, tail)?;
        Ok(super::Started::Backgrounded { id, what })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

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

    #[test]
    fn the_default_window_is_ten_seconds_of_short_polls() {
        let budget = PollBudget::DEFAULT;
        assert_eq!(budget.polls(), 500);
        assert_eq!(budget.interval(), Duration::from_millis(20));
    }
}
