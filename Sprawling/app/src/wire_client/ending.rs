// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, EventKind, RunId};

pub(crate) enum Until {
    Quiet,

    Event(EventKind),

    Run {
        under: Address,
        milestone: Milestone,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum Milestone {
    Started,

    Frozen,
}

impl Milestone {
    fn kind(self) -> EventKind {
        match self {
            Self::Started => EventKind::RunStarted,
            Self::Frozen => EventKind::RunFrozen,
        }
    }
}

pub(crate) struct Heard {
    pub(crate) frames: u32,
    pub(crate) refusals: u32,

    pub(crate) answers: u32,
    pub(crate) awaited: Awaited,

    pub(crate) run: Option<RunId>,
}

#[derive(Clone, Copy)]
pub(crate) enum Awaited {
    Nothing,

    Arrived,

    Missing,
}

pub(crate) enum Spoken {
    Refused,

    Answered,

    Quiet,

    Unfinished,
}

impl Heard {
    pub(crate) fn spoken(&self) -> Spoken {
        match (self.refusals, self.answers, self.awaited) {
            (0, 0, Awaited::Nothing | Awaited::Arrived | Awaited::Missing) => Spoken::Quiet,
            (0, _, Awaited::Missing) => Spoken::Unfinished,
            (0, _, Awaited::Nothing | Awaited::Arrived) => Spoken::Answered,
            (_, _, Awaited::Nothing | Awaited::Arrived | Awaited::Missing) => Spoken::Refused,
        }
    }
}

pub(super) enum Ending {
    Reply,
    Quiet,
    Event(EventKind),

    Run {
        under: Address,
        milestone: Milestone,
        run: Option<RunId>,
    },
}

impl Ending {
    pub(super) fn of(sent: &wire::ClientFrame, until: Until) -> Self {
        match (sent, until) {
            (
                wire::ClientFrame::Ask(_) | wire::ClientFrame::Hello(_),
                Until::Quiet | Until::Event(_) | Until::Run { .. },
            ) => Self::Reply,
            (wire::ClientFrame::Command(_), Until::Quiet) => Self::Quiet,
            (wire::ClientFrame::Command(_), Until::Event(kind)) => Self::Event(kind),
            (wire::ClientFrame::Command(_), Until::Run { under, milestone }) => Self::Run {
                under,
                milestone,
                run: None,
            },

            (wire::ClientFrame::Monitor(_), Until::Quiet | Until::Event(_) | Until::Run { .. }) => {
                Self::Quiet
            }
        }
    }

    pub(super) fn ends_on(&mut self, frame: &Reply) -> bool {
        match (self, frame) {
            (Self::Reply, Reply::Answer | Reply::Refusal)
            | (Self::Event(_) | Self::Run { .. }, Reply::Refusal) => true,
            (Self::Event(awaited), Reply::Event { kind, .. }) => *awaited == *kind,
            (
                Self::Run {
                    under,
                    milestone,
                    run,
                },
                Reply::Event {
                    kind,
                    run: seen,
                    addr,
                },
            ) => {
                if run.is_none()
                    && *kind == EventKind::RunStarted
                    && addr.as_ref().is_some_and(|addr| within(addr, under))
                {
                    *run = Some(*seen);
                }
                *run == Some(*seen) && *kind == milestone.kind()
            }
            (Self::Reply, Reply::Event { .. } | Reply::Other)
            | (Self::Event(_) | Self::Run { .. }, Reply::Answer | Reply::Other)
            | (Self::Quiet, Reply::Answer | Reply::Refusal | Reply::Event { .. } | Reply::Other) => {
                false
            }
        }
    }

    pub(super) fn not_yet(&self) -> Awaited {
        match self {
            Self::Quiet => Awaited::Nothing,
            Self::Reply | Self::Event(_) | Self::Run { .. } => Awaited::Missing,
        }
    }

    pub(super) fn run(&self) -> Option<RunId> {
        match self {
            Self::Run { run, .. } => *run,
            Self::Reply | Self::Quiet | Self::Event(_) => None,
        }
    }

    pub(super) fn echo(&self) -> Echo {
        match self {
            Self::Run {
                milestone: Milestone::Started,
                ..
            } => Echo::Stderr,
            Self::Reply
            | Self::Quiet
            | Self::Event(_)
            | Self::Run {
                milestone: Milestone::Frozen,
                ..
            } => Echo::Stdout,
        }
    }
}

fn within(addr: &Address, under: &Address) -> bool {
    addr.as_str()
        .strip_prefix(under.as_str())
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

pub(super) enum Echo {
    Stdout,
    Stderr,
}

pub(super) enum Reply {
    Answer,
    Refusal,

    Event {
        kind: EventKind,
        run: RunId,
        addr: Option<Address>,
    },
    Other,
}

impl Reply {
    pub(super) fn of(text: &str) -> Self {
        match serde_json::from_str::<wire::ServerFrame>(text) {
            Ok(wire::ServerFrame::Answered(answered)) => match answered.outcome {
                wire::AskOutcome::Answer(_) => Self::Answer,
                wire::AskOutcome::Refusal(_) => Self::Refusal,
            },
            Ok(wire::ServerFrame::Refusal(_)) => Self::Refusal,
            Ok(wire::ServerFrame::Event(record)) => Self::Event {
                kind: record.kind(),
                run: record.run(),
                addr: record.addr().cloned(),
            },
            Ok(
                wire::ServerFrame::Welcome(_)
                | wire::ServerFrame::Delta(_)
                | wire::ServerFrame::Log(_)
                | wire::ServerFrame::Lagged(_)
                | wire::ServerFrame::Output(_)
                | wire::ServerFrame::Monitor(_),
            ) => Self::Other,

            Err(_) => Self::Other,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::{Address, Ending, EventKind, Heard, Milestone, Reply, RunId, Spoken};

    #[test]
    fn a_dispatch_silenced_before_its_milestone_is_unfinished() {
        let mut ending = Ending::Run {
            under: Address::parse("watchtower").unwrap(),
            milestone: Milestone::Frozen,
            run: None,
        };
        assert!(!ending.ends_on(&Reply::Other));
        let heard = Heard {
            frames: 2,
            refusals: 0,
            answers: 1,
            awaited: ending.not_yet(),
            run: ending.run(),
        };
        assert!(matches!(heard.spoken(), Spoken::Unfinished));
    }

    fn run_event(kind: EventKind, run: RunId, addr: &str) -> Reply {
        Reply::Event {
            kind,
            run,
            addr: Some(Address::parse(addr).unwrap()),
        }
    }

    #[test]
    fn a_dispatch_ends_when_its_own_run_freezes() {
        let ours = RunId::from_bytes([1; 16]);
        let theirs = RunId::from_bytes([2; 16]);
        let mut ending = Ending::Run {
            under: Address::parse("watchtower").unwrap(),
            milestone: Milestone::Frozen,
            run: None,
        };
        let heard = [
            run_event(EventKind::RunStarted, theirs, "elsewhere/room"),
            run_event(EventKind::RunStarted, ours, "watchtower/first-look"),
            run_event(EventKind::RunFrozen, theirs, "elsewhere/room"),
            run_event(EventKind::RunFrozen, ours, "watchtower/first-look"),
        ]
        .iter()
        .map(|reply| ending.ends_on(reply))
        .collect::<Vec<_>>();
        assert_eq!(heard, [false, false, false, true]);
        assert_eq!(ending.run(), Some(ours));
    }

    #[test]
    fn a_detached_dispatch_ends_when_its_run_starts() {
        let ours = RunId::from_bytes([1; 16]);
        let mut ending = Ending::Run {
            under: Address::parse("watchtower/first-look").unwrap(),
            milestone: Milestone::Started,
            run: None,
        };
        let ended = ending.ends_on(&run_event(
            EventKind::RunStarted,
            ours,
            "watchtower/first-look",
        ));
        assert_eq!((ended, ending.run()), (true, Some(ours)));
    }
}
