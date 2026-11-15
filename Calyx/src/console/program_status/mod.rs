// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fmt;

use base64::Engine as _;
use kernel::{Completion, EventKind, EventRecord};

const APP: &str = "sprawling";

const SWITCH: &str = "SPRAWLING_PROGRAM_STATUS";

const NEVER: &str = "never";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reporting {
    On,
    Off,
}

impl Reporting {
    pub(crate) fn from_environment() -> Reporting {
        deciding(std::env::var(SWITCH).ok().as_deref())
    }
}

fn deciding(variable: Option<&str>) -> Reporting {
    match variable {
        Some(NEVER) => Reporting::Off,
        Some(_) | None => Reporting::On,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rest {
    Idle,
    Done,
    Failed,
}

impl Rest {
    pub(crate) fn after(record: &EventRecord) -> Result<Option<Rest>, kernel::AxError> {
        if record.kind() != EventKind::RunFrozen {
            return Ok(None);
        }
        let frozen = record.data().read::<kernel::event::record::RunFrozen>()?;
        Ok(Some(Rest::of_completion(&frozen.completion)))
    }

    fn of_completion(word: &str) -> Rest {
        if word == Completion::Cancelled.name() {
            Rest::Idle
        } else if word == Completion::Limit.name() {
            Rest::Failed
        } else {
            Rest::Done
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) runs: u64,
    pub(crate) approvals: u64,
}

impl Counts {
    pub(crate) fn of(vitals: &wire::MetricsAnswer) -> Counts {
        Counts {
            runs: vitals.runs_active,
            approvals: vitals.approvals_waiting,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Resting(Rest),
    Working { runs: u64 },
    Blocked { approvals: u64 },
}

pub(crate) fn judged(counts: Counts, rest: Rest) -> State {
    if counts.approvals > 0 {
        State::Blocked {
            approvals: counts.approvals,
        }
    } else if counts.runs > 0 {
        State::Working { runs: counts.runs }
    } else {
        State::Resting(rest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    Ended(Rest),
    Counted(Counts),
    Closed,
}

pub(crate) fn moves(kind: EventKind) -> bool {
    kind == EventKind::RunStarted
        || kind == EventKind::RunFrozen
        || kind == EventKind::ApprovalRequested
        || kind == EventKind::ApprovalResolved
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shown {
    Nothing,
    Reported(State),
    Cleared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Held {
    pub(crate) rest: Rest,
    pub(crate) shown: Shown,
}

impl Held {
    pub(crate) const OPENING: Held = Held {
        rest: Rest::Idle,
        shown: Shown::Nothing,
    };
}

pub(crate) fn step(held: Held, event: Event) -> (Held, Option<Report>) {
    match (held.shown, event) {
        (Shown::Cleared, _) => (held, None),
        (_, Event::Ended(rest)) => (Held { rest, ..held }, None),
        (shown, Event::Counted(counts)) => {
            let state = judged(counts, held.rest);
            if shown == Shown::Reported(state) {
                (held, None)
            } else {
                (
                    Held {
                        shown: Shown::Reported(state),
                        ..held
                    },
                    Some(Report::Of(state)),
                )
            }
        }
        (Shown::Nothing, Event::Closed) => (
            Held {
                shown: Shown::Cleared,
                ..held
            },
            None,
        ),
        (Shown::Reported(_), Event::Closed) => (
            Held {
                shown: Shown::Cleared,
                ..held
            },
            Some(Report::Cleared),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Report {
    Of(State),
    Cleared,
}

impl Report {
    fn pairs(self) -> (&'static str, Option<&'static str>) {
        match self {
            Report::Of(State::Resting(Rest::Idle)) => ("idle", None),
            Report::Of(State::Resting(Rest::Done)) => ("done", None),
            Report::Of(State::Resting(Rest::Failed)) => ("error", None),
            Report::Of(State::Working { .. }) => ("working", None),
            Report::Of(State::Blocked { .. }) => ("blocked", Some("permission")),
            Report::Cleared => ("clear", None),
        }
    }

    fn message(self) -> Option<String> {
        match self {
            Report::Of(State::Working { runs: 1 }) => Some("1 run is going".to_owned()),
            Report::Of(State::Working { runs }) => Some(format!("{runs} runs are going")),
            Report::Of(State::Blocked { approvals: 1 }) => {
                Some("1 request waits for an answer".to_owned())
            }
            Report::Of(State::Blocked { approvals }) => {
                Some(format!("{approvals} requests wait for an answer"))
            }
            Report::Of(State::Resting(Rest::Done)) => Some("every run has finished".to_owned()),
            Report::Of(State::Resting(Rest::Failed)) => {
                Some("a run stopped at its limit".to_owned())
            }
            Report::Of(State::Resting(Rest::Idle)) | Report::Cleared => None,
        }
    }
}

impl crossterm::Command for Report {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        let (state, kind) = self.pairs();
        write!(f, "\x1b]7501;state={state}")?;
        if let Report::Of(_) = self {
            write!(f, ":app={APP}")?;
        }
        if let Some(kind) = kind {
            write!(f, ":kind={kind}")?;
        }
        if let Some(message) = self.message() {
            let encoded = base64::engine::general_purpose::STANDARD.encode(message);
            write!(f, ":msg={encoded}")?;
        }
        f.write_str("\x1b\\")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
