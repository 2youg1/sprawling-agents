// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading one text frame off a live socket, and the verdict on a frame
//! this build cannot read.
//!
//! **A frame that will not decode is a disagreement about the wire, not
//! a broken connection.** The two ends differ by a version, a variant
//! or a field, and the peer that hears nothing about it reconnects for
//! ever: the page a person is looking at stays blank while the socket
//! comes up, greets, fails on the same frame and closes again. So the
//! refusal carries [`AxCode::WireMismatch`] and says which wire this
//! server speaks, which is what turns the fault into one reload.
//!
//! The session counts what it could not read so a repeat is visible in
//! the refusal a person is shown, and it stays open: the peer has been
//! told, and a told peer stops.

use kernel::{AxCode, AxError};

use super::SessionStep;
use crate::wire::{ClientFrame, WIRE_V};

/// One session's reader, and the tally of frames it could not read.
///
/// Held by the session task, so the count is per peer rather than per
/// process: it answers "this connection keeps sending me frames I
/// cannot read", which is the question a person debugging a stale page
/// has.
#[derive(Debug, Default)]
pub(crate) struct Inbound {
    unreadable: u32,
}

impl Inbound {
    /// A session that has read nothing yet.
    pub(crate) const fn new() -> Self {
        Self { unreadable: 0 }
    }

    /// Reads one text frame.
    ///
    /// `Err` is the step the shell carries out instead: the refusal to
    /// send, and whether the session ends. The shell therefore handles
    /// an unreadable frame and a decided frame through the same branch,
    /// so both refusals travel the one path.
    ///
    /// # Errors
    /// [`AxCode::WireMismatch`] when this build cannot read the text as
    /// a [`ClientFrame`]. The session stays open.
    pub(crate) fn read(&mut self, text: &str) -> Result<ClientFrame, SessionStep> {
        match serde_json::from_str::<ClientFrame>(text) {
            Ok(frame) => Ok(frame),
            Err(_) => Err(self.refuse()),
        }
    }

    /// Counts one unreadable frame and states the refusal for it.
    ///
    /// The subject names the count and this server's wire version, and
    /// never the bytes that arrived: a frame from a peer is that peer's
    /// content, and echoing it puts whatever it carried into a log.
    fn refuse(&mut self) -> SessionStep {
        self.unreadable = self.unreadable.saturating_add(1);
        let seen = self.unreadable;
        SessionStep::Refuse {
            error: Box::new(
                AxError::failure(
                    AxCode::WireMismatch,
                    "read a client frame",
                    format!(
                        "frame {seen} of this session is not one this server can read; it \
                         speaks wire v{WIRE_V}"
                    ),
                )
                .with_recovery("reload the page to fetch the client this server was built with"),
            ),
            // The peer is told rather than disconnected: a page that
            // only sees the socket close cannot tell a wire mismatch
            // from a lost network, and it reconnects into the same
            // mismatch every time.
            close: false,
        }
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
mod tests {
    use super::*;
    use crate::wire::{Query, schema_hash};

    /// The frame a client one wire version ahead sends: this build has
    /// no reading for it, and what the peer must be told is that the
    /// two ends differ — never that the connection failed.
    #[test]
    fn a_frame_from_a_later_wire_is_refused_rather_than_disconnected() {
        let mut inbound = Inbound::new();
        let later = r#"{"hello":{"wire_v":32,"schema":{"later":true},"token":null}}"#;
        let Err(SessionStep::Refuse { error, close }) = inbound.read(later) else {
            panic!("a frame this build cannot read is refused");
        };
        assert_eq!(*error.code(), AxCode::WireMismatch);
        assert!(
            !close,
            "the session stays open; closing is what a page reads as an outage"
        );
        assert!(error.recovery().contains("reload"));
    }

    /// The tally is what makes a peer stuck on the wrong wire visible:
    /// the same refusal arriving again says which repeat it is.
    #[test]
    fn every_unreadable_frame_is_counted_in_what_the_peer_is_told() {
        let mut inbound = Inbound::new();
        let subjects: Vec<String> = (0..2)
            .map(|_| {
                let Err(SessionStep::Refuse { error, .. }) = inbound.read("{}") else {
                    panic!("an empty object is not a frame");
                };
                error.subject().to_owned()
            })
            .collect();
        assert!(subjects[0].contains("frame 1"));
        assert!(subjects[1].contains("frame 2"));
    }

    /// A readable frame is handed on untouched, and nothing is counted.
    #[test]
    fn a_frame_this_build_reads_is_handed_on() {
        let mut inbound = Inbound::new();
        let hello = format!(
            r#"{{"hello":{{"wire_v":{WIRE_V},"schema":"{}","token":null}}}}"#,
            schema_hash()
        );
        assert!(matches!(
            inbound.read(&hello),
            Ok(ClientFrame::Hello(said)) if said.wire_v == WIRE_V
        ));
        assert!(matches!(
            inbound.read(r#"{"query":"city_view"}"#),
            Ok(ClientFrame::Query(Query::CityView))
        ));
        assert_eq!(inbound.unreadable, 0);
    }
}
