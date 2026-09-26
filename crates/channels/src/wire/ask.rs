// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A question and its answer, paired by the number the asking side
//! minted rather than by what the answer happens to contain.
//!
//! The server echoes an [`AskId`] and never judges it: pairing is the
//! page's concern, and a server that checked uniqueness would be a second
//! place deciding which answer belongs to which question.

use kernel::{AxError, Seq};
use serde::{Deserialize, Serialize};

use super::{Answer, Query};

/// The asking side's own number for one question, minted monotonically
/// per connection and echoed on the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AskId(pub u32);

/// One question, carrying the number its answer will come back under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Ask {
    pub ask_id: AskId,
    pub query: Query,
}

/// The reply to one [`Ask`].
///
/// `as_of` is the first seq the answer does not reflect: the answer holds
/// every record before it and none from it on. It and the answer are read
/// under the same view lock, and a view that has folded nothing answers
/// with `Seq::FIRST`, so genesis is never mistaken for already folded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Answered {
    pub ask_id: AskId,
    pub as_of: Seq,
    pub outcome: AskOutcome,
}

/// Whether the question was answered or refused. A refusal of a question
/// travels here rather than as a bare `Refusal` frame, so the page knows
/// which question fell through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AskOutcome {
    Answer(Answer),
    Refusal(AxError),
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::super::{ClientFrame, Query};
    use super::{Ask, AskId};

    #[test]
    fn a_client_frame_round_trips_through_json() {
        let frame = ClientFrame::Ask(Ask {
            ask_id: AskId(7),
            query: Query::CityView,
        });
        let text = serde_json::to_string(&frame).unwrap();
        let back: ClientFrame = serde_json::from_str(&text).unwrap();
        assert_eq!(frame, back);
    }
}
