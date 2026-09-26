// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! When `sprawling call` stops listening (sprawling-SPEC.md section
//! 8-10): a query ends on its one reply, a command on the city's quiet
//! or on the event its caller named.

use super::{Awaited, Until};
use kernel::EventKind;

/// When a call stops listening, decided once by the kind of frame sent
/// and what its caller waits for.
///
/// A frame that has exactly one reply ends on that reply; a command's
/// consequences are the city's business, so a command ends when the
/// city has been quiet for the window, unless its caller named the
/// event that finishes the work. The window still bounds a reply or an
/// event that never comes.
pub(super) enum Ending {
    OnReply,
    OnQuiet,
    OnEvent(EventKind),
}

impl Ending {
    /// A query is answered once, and a greeting on a live session is
    /// refused once (`channels::reception`), so both have one reply.
    pub(super) fn of(sent: &channels::ClientFrame, until: Until) -> Self {
        match (sent, until) {
            (
                channels::ClientFrame::Ask(_) | channels::ClientFrame::Hello(_),
                Until::Quiet | Until::Event(_),
            ) => Self::OnReply,
            (channels::ClientFrame::Command(_), Until::Quiet) => Self::OnQuiet,
            (channels::ClientFrame::Command(_), Until::Event(kind)) => Self::OnEvent(kind),
        }
    }

    /// A refused command causes no event, so a refusal ends a call that
    /// waits for one.
    pub(super) fn ends_on(&self, frame: &Reply) -> bool {
        match (self, frame) {
            (Self::OnReply, Reply::Answer | Reply::Refusal)
            | (Self::OnEvent(_), Reply::Refusal) => true,
            (Self::OnEvent(awaited), Reply::Event(kind)) => awaited == kind,
            (Self::OnReply, Reply::Event(_) | Reply::Other)
            | (Self::OnEvent(_), Reply::Answer | Reply::Other)
            | (Self::OnQuiet, Reply::Answer | Reply::Refusal | Reply::Event(_) | Reply::Other) => {
                false
            }
        }
    }

    /// What was heard of the awaited frame while it has not come: a
    /// call that awaits nothing but the quiet has nothing to miss.
    pub(super) fn not_yet(&self) -> Awaited {
        match self {
            Self::OnQuiet => Awaited::Nothing,
            Self::OnReply | Self::OnEvent(_) => Awaited::Missing,
        }
    }
}

/// What one frame from the city is, as far as counting and ending go.
///
/// Read back through the wire's own type rather than by looking for a
/// word in the text, so a payload that merely mentions refusal is not
/// counted as one.
pub(super) enum Reply {
    Answer,
    Refusal,
    Event(EventKind),
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
            Ok(channels::ServerFrame::Event(record)) => Self::Event(record.kind()),
            Ok(
                channels::ServerFrame::Welcome(_)
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
