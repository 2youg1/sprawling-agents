// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! When a wire client stops listening, and what it heard by then
//! (sprawling-SPEC.md section 8-10): a query ends on its one reply, a
//! command on the city's quiet or on the event its caller named, and a
//! dispatch on its own run's milestone.

use kernel::{Address, EventKind, RunId};

/// What a call waits for, as its caller says it. A query ends on its
/// one reply whatever this says, because nothing follows that reply.
pub(crate) enum Until {
    /// The city going quiet for the window: the caller does not know
    /// which event finishes the work.
    Quiet,
    /// The first event of this kind, or a refusal, since a refused
    /// command causes no event at all.
    Event(EventKind),
    /// The run started at or under `under` reaching `milestone`, or a
    /// refusal. The city names the run only when it starts it, so the
    /// caller names where it runs instead.
    Run {
        under: Address,
        milestone: Milestone,
    },
}

/// Which point in a dispatched run's life ends the wait.
#[derive(Clone, Copy)]
pub(crate) enum Milestone {
    /// `--detach`: the run exists.
    Started,
    /// The run has frozen, however it ended.
    Frozen,
}

impl Milestone {
    /// The event that marks this point, so a milestone is read through
    /// the same event kinds `--until` names.
    fn kind(self) -> EventKind {
        match self {
            Self::Started => EventKind::RunStarted,
            Self::Frozen => EventKind::RunFrozen,
        }
    }
}

/// What came back before the client stopped listening.
pub(crate) struct Heard {
    pub(crate) frames: u32,
    pub(crate) refusals: u32,
    /// Frames that arrived *after* the frame was sent.
    ///
    /// Counted apart from `frames` because the handshake's `Welcome` is
    /// a frame too, so `frames` is never zero and cannot tell silence
    /// from an answer. Subtracting one at the caller would copy the
    /// shape of the handshake into a second place.
    pub(crate) answers: u32,
    pub(crate) awaited: Awaited,
    /// The run a wait on [`Until::Run`] saw start.
    pub(crate) run: Option<RunId>,
}

/// What became of the frame a call was waiting for.
#[derive(Clone, Copy)]
pub(crate) enum Awaited {
    /// The call waits only for the city's quiet.
    Nothing,
    /// The reply, the named event or the run's milestone arrived and
    /// ended the call.
    Arrived,
    /// The window closed first: the city may still be working.
    Missing,
}

/// What the city did about the frame it was sent. Exhaustive: these
/// four are what a caller can learn inside one quiet window, and the
/// exit codes of `sprawling call` and `sprawling dispatch` are one table
/// over them.
pub(crate) enum Spoken {
    /// The city refused, inside the window.
    Refused,
    /// The city answered, inside the window, and refused nothing.
    Answered,
    /// The frame went out and nothing came back before the window
    /// closed. Whether the city took the work is not knowable here, so
    /// this is neither of the other two.
    Quiet,
    /// The city spoke, but what the call waits for - a query's reply, a
    /// named event, a dispatched run's milestone - did not come before
    /// the window closed: the work may still be going on, or may never
    /// have started.
    Unfinished,
}

impl Heard {
    /// Refusal first, because a refusal that also carried events is
    /// still a refusal; then silence, which is nothing at all arriving
    /// after the frame; then the awaited frame missing, since events
    /// that are not the one asked for do not say the work is done.
    pub(crate) fn spoken(&self) -> Spoken {
        match (self.refusals, self.answers, self.awaited) {
            (0, 0, Awaited::Nothing | Awaited::Arrived | Awaited::Missing) => Spoken::Quiet,
            (0, _, Awaited::Missing) => Spoken::Unfinished,
            (0, _, Awaited::Nothing | Awaited::Arrived) => Spoken::Answered,
            (_, _, Awaited::Nothing | Awaited::Arrived | Awaited::Missing) => Spoken::Refused,
        }
    }
}

/// When a call stops listening, decided once by the kind of frame sent
/// and what its caller waits for.
///
/// A frame that has exactly one reply ends on that reply; a command's
/// consequences are the city's business, so a command ends when the
/// city has been quiet for the window, unless its caller named the
/// event or the run milestone that finishes the work. The window still
/// bounds a reply, an event or a milestone that never comes.
pub(super) enum Ending {
    Reply,
    Quiet,
    Event(EventKind),
    /// `run` is the run started at or under `under`, once its
    /// `run_started` has arrived.
    Run {
        under: Address,
        milestone: Milestone,
        run: Option<RunId>,
    },
}

impl Ending {
    /// A query is answered once, and a greeting on a live session is
    /// refused once (`channels::reception`), so both have one reply.
    pub(super) fn of(sent: &channels::ClientFrame, until: Until) -> Self {
        match (sent, until) {
            (
                channels::ClientFrame::Ask(_) | channels::ClientFrame::Hello(_),
                Until::Quiet | Until::Event(_) | Until::Run { .. },
            ) => Self::Reply,
            (channels::ClientFrame::Command(_), Until::Quiet) => Self::Quiet,
            (channels::ClientFrame::Command(_), Until::Event(kind)) => Self::Event(kind),
            (channels::ClientFrame::Command(_), Until::Run { under, milestone }) => Self::Run {
                under,
                milestone,
                run: None,
            },
        }
    }

    /// Reads one frame and says whether listening ends with it. A
    /// refused command causes no event, so a refusal ends every wait for
    /// one. An [`Ending::Run`] also learns its run here, from the first
    /// `run_started` at or under the address it dispatched to, so another
    /// run reaching the same milestone does not end it.
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

    /// What was heard of the awaited frame while it has not come: a
    /// call that awaits nothing but the quiet has nothing to miss.
    pub(super) fn not_yet(&self) -> Awaited {
        match self {
            Self::Quiet => Awaited::Nothing,
            Self::Reply | Self::Event(_) | Self::Run { .. } => Awaited::Missing,
        }
    }

    /// The run an [`Ending::Run`] saw start.
    pub(super) fn run(&self) -> Option<RunId> {
        match self {
            Self::Run { run, .. } => *run,
            Self::Reply | Self::Quiet | Self::Event(_) => None,
        }
    }

    /// Where the frames go: `--detach` keeps stdout for the run id alone.
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

/// Whether `addr` is `under` or a room beneath it: a dispatch to a
/// building runs in a room the city names.
fn within(addr: &Address, under: &Address) -> bool {
    addr.as_str()
        .strip_prefix(under.as_str())
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The stream every frame heard is printed on.
pub(super) enum Echo {
    Stdout,
    Stderr,
}

/// What one frame from the city is, as far as counting and ending go.
///
/// Read back through the wire's own type rather than by looking for a
/// word in the text, so a payload that merely mentions refusal is not
/// counted as one.
pub(super) enum Reply {
    Answer,
    Refusal,
    /// An event: its kind is what `--until` names, and its run and the
    /// address that run ran at are how a dispatch tells its own run from
    /// another.
    Event {
        kind: EventKind,
        run: RunId,
        addr: Option<Address>,
    },
    Other,
}

impl Reply {
    pub(super) fn of(text: &str) -> Self {
        match serde_json::from_str::<channels::ServerFrame>(text) {
            Ok(channels::ServerFrame::Answered(answered)) => match answered.outcome {
                channels::AskOutcome::Answer(_) => Self::Answer,
                channels::AskOutcome::Refusal(_) => Self::Refusal,
            },
            Ok(channels::ServerFrame::Refusal(_)) => Self::Refusal,
            Ok(channels::ServerFrame::Event(record)) => Self::Event {
                kind: record.kind(),
                run: record.run(),
                addr: record.addr().cloned(),
            },
            Ok(
                channels::ServerFrame::Welcome(_)
                | channels::ServerFrame::Delta(_)
                | channels::ServerFrame::Log(_)
                | channels::ServerFrame::Lagged(_)
                | channels::ServerFrame::Output(_),
            ) => Self::Other,
            // A frame this build cannot read is still printed; it is
            // simply not one that ends the call.
            Err(_) => Self::Other,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::{Address, Ending, EventKind, Heard, Milestone, Reply, RunId, Spoken};

    /// A dispatch that heard only unrelated frames before the window
    /// closed did not see its run freeze, so it is not an answer.
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

    /// A dispatch into a building waits for the run that started in one
    /// of its rooms, and another run freezing meanwhile does not end it.
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

    /// `--detach` ends on the run's start, with the run known.
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
