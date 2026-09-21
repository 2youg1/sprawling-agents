// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A signal id: what a duplicate delivery is recognised by.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

/// A signal's identity, and the thing duplicates are recognised by.
///
/// It serializes as the string [`SignalId::parse`] accepted and reads
/// back through that same parse, so a ledger line carrying an id meets
/// the grammar once rather than once per reader.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SignalId(String);

impl TryFrom<String> for SignalId {
    type Error = AxError;

    fn try_from(raw: String) -> Result<SignalId, AxError> {
        SignalId::parse(&raw)
    }
}

impl From<SignalId> for String {
    fn from(id: SignalId) -> String {
        id.0
    }
}

impl SignalId {
    /// # Errors
    /// Refuses an empty id and one carrying whitespace: an id is
    /// compared, logged and replayed, and all three go wrong quietly
    /// when it can contain a space.
    pub fn parse(raw: &str) -> Result<SignalId, AxError> {
        if raw.is_empty() || raw.chars().any(char::is_whitespace) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a signal id",
                format!("{raw:?}"),
            )
            .with_recovery(
                "use a non-empty id with no whitespace, such as a run id and a counter",
            ));
        }
        Ok(SignalId(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
