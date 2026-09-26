// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! When a wire client stops listening, and what it heard by then
//! (sprawling-SPEC.md section 8-10): a query ends on its one reply, a
//! command on the city's quiet, and a dispatch on its run's milestone.

use kernel::{Address, EventKind, RunId};

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
    /// The run a dispatch started, when an [`Ending::OnRun`] saw it.
    pub(crate) run: Option<RunId>,
    /// Whether the wait stopped on its milestone or on a silence.
    pub(crate) watch: Watch,
}

/// How an [`Ending::OnRun`] wait stopped. A frame or a reply ending the
/// other two endings carries no milestone, so they are `NotAsked`.
#[derive(Clone, Copy)]
pub(crate) enum Watch {
    NotAsked,
    Reached,
    /// The quiet window closed before the run reached its milestone.
    Unreached,
}

/// What stopped the listening loop.
#[derive(Clone, Copy)]
pub(super) enum Stopped {
    /// [`Ending::ends_on`] said so.
    OnFrame,
    /// The quiet window closed, or the city closed the socket.
    OnSilence,
}

/// What the city did about the frame it was sent. Exhaustive: these
/// three are what a caller can learn inside one quiet window, and the
/// exit code `sprawling call` returns is one per arm.
pub(crate) enum Spoken {
    /// The city refused, inside the window.
    Refused,
    /// The city answered, inside the window, and refused nothing.
    Answered,
    /// The frame went out and nothing came back before the window
    /// closed. Whether the city took the work is not knowable here, so
    /// this is neither of the other two.
    Quiet,
    /// The city spoke, but the dispatched run did not reach the
    /// milestone the caller waits on before the window closed: the run
    /// may still be working, or may never have started.
    Unfinished,
}

impl Heard {
    /// Refusal first, because a refusal that also carried events is
    /// still a refusal; silence last, because it is only silence when
    /// nothing at all arrived.
    pub(crate) fn spoken(&self) -> Spoken {
        match (self.refusals, self.answers) {
            (0, 0) => Spoken::Quiet,
            (0, _) => Spoken::Answered,
            _ => Spoken::Refused,
        }
    }
}

/// When a client stops listening.
///
/// A frame that has exactly one reply ends on that reply; a command's
/// consequences are the city's business, so a command ends when the
/// city has been quiet for the window. A dispatch whose caller waits on
/// its run ends on that run's milestone. The window still bounds a
/// reply that never comes.
pub(crate) enum Ending {
    OnReply,
    OnQuiet,
    /// The run started under `under` reaches `until`; `run` is that run
    /// once its `run_started` has arrived.
    OnRun {
        under: Address,
        until: Milestone,
        run: Option<RunId>,
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

impl Ending {
    /// A query is answered once, and a greeting on a live session is
    /// refused once (`channels::reception`), so both have one reply.
    pub(crate) fn of(sent: &channels::ClientFrame) -> Self {
        match sent {
            channels::ClientFrame::Query(_) | channels::ClientFrame::Hello(_) => Self::OnReply,
            channels::ClientFrame::Command(_) => Self::OnQuiet,
        }
    }

    /// Reads one frame and says whether listening ends with it. An
    /// [`Ending::OnRun`] also learns its run here, from the first
    /// `run_started` at or under the address it dispatched to.
    pub(super) fn ends_on(&mut self, frame: &Reply) -> bool {
        match (self, frame) {
            (Self::OnReply, Reply::Answer | Reply::Refusal)
            | (Self::OnRun { .. }, Reply::Refusal) => true,
            (Self::OnReply, Reply::Other | Reply::Run { .. })
            | (Self::OnQuiet, Reply::Answer | Reply::Refusal | Reply::Other | Reply::Run { .. })
            | (Self::OnRun { .. }, Reply::Answer | Reply::Other) => false,
            (
                Self::OnRun { under, until, run },
                Reply::Run {
                    reached,
                    run: seen,
                    addr,
                },
            ) => match (*run, reached) {
                (None, Milestone::Started) if addr.as_ref().is_some_and(|a| within(a, under)) => {
                    *run = Some(*seen);
                    match until {
                        Milestone::Started => true,
                        Milestone::Frozen => false,
                    }
                }
                (Some(ours), Milestone::Frozen) => ours == *seen,
                (None, Milestone::Started | Milestone::Frozen) | (Some(_), Milestone::Started) => {
                    false
                }
            },
        }
    }

    /// How the wait stopped, as far as a run's milestone goes.
    pub(super) fn watch(&self, stopped: Stopped) -> Watch {
        match (self, stopped) {
            (Self::OnReply | Self::OnQuiet, Stopped::OnFrame | Stopped::OnSilence) => {
                Watch::NotAsked
            }
            (Self::OnRun { .. }, Stopped::OnFrame) => Watch::Reached,
            (Self::OnRun { .. }, Stopped::OnSilence) => Watch::Unreached,
        }
    }

    /// The run an [`Ending::OnRun`] saw start.
    pub(super) fn run(&self) -> Option<RunId> {
        match self {
            Self::OnRun { run, .. } => *run,
            Self::OnReply | Self::OnQuiet => None,
        }
    }

    /// Where the frames go: `--detach` keeps stdout for the run id alone.
    pub(super) fn echo(&self) -> Echo {
        match self {
            Self::OnRun {
                until: Milestone::Started,
                ..
            } => Echo::Stderr,
            Self::OnReply | Self::OnQuiet | Self::OnRun { .. } => Echo::Stdout,
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
    /// A run reached a milestone, with where it ran.
    Run {
        reached: Milestone,
        run: RunId,
        addr: Option<Address>,
    },
    Other,
}

impl Reply {
    fn run(reached: Milestone, record: &kernel::EventRecord) -> Self {
        Self::Run {
            reached,
            run: record.run(),
            addr: record.addr().cloned(),
        }
    }

    pub(super) fn of(text: &str) -> Self {
        match serde_json::from_str::<channels::ServerFrame>(text) {
            Ok(channels::ServerFrame::Answer(_)) => Self::Answer,
            Ok(channels::ServerFrame::Refusal(_)) => Self::Refusal,
            // Two kinds out of the whole event vocabulary end a wait;
            // every other event is printed and passed over.
            Ok(channels::ServerFrame::Event(record)) if record.kind() == EventKind::RunStarted => {
                Self::run(Milestone::Started, &record)
            }
            Ok(channels::ServerFrame::Event(record)) if record.kind() == EventKind::RunFrozen => {
                Self::run(Milestone::Frozen, &record)
            }
            Ok(
                channels::ServerFrame::Welcome(_)
                | channels::ServerFrame::Event(_)
                | channels::ServerFrame::Delta(_)
                | channels::ServerFrame::Log(_)
                | channels::ServerFrame::Lagged(_),
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
    use super::{Address, Ending, Heard, Milestone, Reply, RunId, Spoken, Stopped};

    /// A dispatch that heard only unrelated frames before the window
    /// closed did not see its run freeze, so it is not an answer.
    #[test]
    fn a_dispatch_silenced_before_its_milestone_is_unfinished() {
        let mut ending = Ending::OnRun {
            under: Address::parse("watchtower").unwrap(),
            until: Milestone::Frozen,
            run: None,
        };
        assert!(!ending.ends_on(&Reply::Other));
        let heard = Heard {
            frames: 2,
            refusals: 0,
            answers: 1,
            run: ending.run(),
            watch: ending.watch(Stopped::OnSilence),
        };
        assert!(matches!(heard.spoken(), Spoken::Unfinished));
    }

    fn run_event(reached: Milestone, run: RunId, addr: &str) -> Reply {
        Reply::Run {
            reached,
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
        let mut ending = Ending::OnRun {
            under: Address::parse("watchtower").unwrap(),
            until: Milestone::Frozen,
            run: None,
        };
        let heard = [
            run_event(Milestone::Started, theirs, "elsewhere/room"),
            run_event(Milestone::Started, ours, "watchtower/first-look"),
            run_event(Milestone::Frozen, theirs, "elsewhere/room"),
            run_event(Milestone::Frozen, ours, "watchtower/first-look"),
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
        let mut ending = Ending::OnRun {
            under: Address::parse("watchtower/first-look").unwrap(),
            until: Milestone::Started,
            run: None,
        };
        let ended = ending.ends_on(&run_event(
            Milestone::Started,
            ours,
            "watchtower/first-look",
        ));
        assert_eq!((ended, ending.run()), (true, Some(ours)));
    }
}
