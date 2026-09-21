// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What kind of communication a signal is, and its wire name.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

/// What kind of communication a signal is. `Steer` is a fourth kind
/// rather than a flag beside the other three: it is the only one that
/// overtakes, and urgency has to belong to the signal for one id to
/// always take one lane.
///
/// Its serde form goes through [`SignalKind::as_str`] and
/// [`SignalKind::parse`] rather than through a derived renaming, so the
/// four wire words are spelled in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
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

impl TryFrom<String> for SignalKind {
    type Error = AxError;

    fn try_from(raw: String) -> Result<SignalKind, AxError> {
        SignalKind::parse(&raw)
    }
}

impl From<SignalKind> for String {
    fn from(kind: SignalKind) -> String {
        kind.as_str().to_owned()
    }
}
