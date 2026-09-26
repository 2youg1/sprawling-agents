// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! When `sprawling call` stops listening (sprawling-SPEC.md section
//! 8-10): a query ends on its one reply, a command on the city's quiet.

/// When a call stops listening, decided once by the kind of frame sent.
///
/// A frame that has exactly one reply ends on that reply; a command's
/// consequences are the city's business, so a command ends when the
/// city has been quiet for the window. The window still bounds a reply
/// that never comes.
pub(super) enum Ending {
    OnReply,
    OnQuiet,
}

impl Ending {
    /// A query is answered once, and a greeting on a live session is
    /// refused once (`channels::reception`), so both have one reply.
    pub(super) fn of(sent: &channels::ClientFrame, _until: super::Until) -> Self {
        match sent {
            channels::ClientFrame::Query(_) | channels::ClientFrame::Hello(_) => Self::OnReply,
            channels::ClientFrame::Command(_) => Self::OnQuiet,
        }
    }

    pub(super) fn ends_on(&self, frame: &Reply) -> bool {
        match (self, frame) {
            (Self::OnReply, Reply::Answer | Reply::Refusal) => true,
            (Self::OnReply, Reply::Other)
            | (Self::OnQuiet, Reply::Answer | Reply::Refusal | Reply::Other) => false,
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
    Other,
}

impl Reply {
    pub(super) fn of(text: &str) -> Self {
        match serde_json::from_str::<channels::ServerFrame>(text) {
            Ok(channels::ServerFrame::Answer(_)) => Self::Answer,
            Ok(channels::ServerFrame::Refusal(_)) => Self::Refusal,
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
