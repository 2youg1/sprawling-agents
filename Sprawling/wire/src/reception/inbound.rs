// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

use super::SessionStep;
use crate::frames::{ClientFrame, WIRE_V};

#[derive(Debug, Default)]
pub(crate) struct Inbound {
    unreadable: u32,
}

impl Inbound {
    pub(crate) const fn new() -> Self {
        Self { unreadable: 0 }
    }

    pub(crate) fn read(&mut self, text: &str) -> Result<ClientFrame, SessionStep> {
        match serde_json::from_str::<ClientFrame>(text) {
            Ok(frame) => Ok(frame),
            Err(_) => Err(self.refuse()),
        }
    }

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
    use crate::frames::{Ask, AskId, Query, schema_hash};

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
            inbound.read(r#"{"ask":{"ask_id":7,"query":"city_view"}}"#),
            Ok(ClientFrame::Ask(Ask {
                ask_id: AskId(7),
                query: Query::CityView
            }))
        ));
        assert_eq!(inbound.unreadable, 0);
    }
}
