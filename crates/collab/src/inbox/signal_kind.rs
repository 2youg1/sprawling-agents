// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What kind of communication a signal is, and its wire name.

use kernel::{AxCode, AxError};

/// What kind of communication a signal is. `Steer` is a fourth kind
/// rather than a flag beside the other three: it is the only one that
/// overtakes, and urgency has to belong to the signal for one id to
/// always take one lane.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    Mention,
    Thread,
    Broadcast,
    Steer,
}

impl SignalKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SignalKind::Mention => "mention",
            SignalKind::Thread => "thread",
            SignalKind::Broadcast => "broadcast",
            SignalKind::Steer => "steer",
        }
    }

    /// # Errors
    /// Refuses a kind this version does not know.
    pub fn parse(raw: &str) -> Result<SignalKind, AxError> {
        match raw {
            "mention" => Ok(SignalKind::Mention),
            "thread" => Ok(SignalKind::Thread),
            "broadcast" => Ok(SignalKind::Broadcast),
            "steer" => Ok(SignalKind::Steer),
            other => {
                Err(
                    AxError::failure(AxCode::InvalidArgs, "read a signal kind", other.to_owned())
                        .with_recovery("mention, thread, broadcast or steer"),
                )
            }
        }
    }
}
