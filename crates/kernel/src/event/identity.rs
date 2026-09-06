// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Event identity: run, sequence, and time.

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

/// Run identity; uuid v7 for humans, nil for the city itself. No
/// generation here — the assembly layer (or a seeded simulator) mints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(uuid::Uuid);

impl RunId {
    /// City-level records (genesis, tail truncation) carry the nil id;
    /// real runs use uuid v7, whose timestamp bits never collide with nil.
    pub const CITY: RunId = RunId(uuid::Uuid::nil());

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        RunId(uuid::Uuid::from_bytes(bytes))
    }

    pub fn parse(raw: &str) -> Result<Self, AxError> {
        uuid::Uuid::parse_str(raw).map(RunId).map_err(|_| {
            AxError::failure(AxCode::InvalidArgs, "parse run id", raw)
                .with_recovery("use a hyphenated uuid")
        })
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.hyphenated().fmt(f)
    }
}

/// Event sequence number; contiguous from [`Seq::FIRST`], checked arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seq(u64);

impl Seq {
    /// The genesis line's number.
    pub const FIRST: Seq = Seq(0);

    pub fn new(value: u64) -> Self {
        Seq(value)
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn next(self) -> Result<Seq, AxError> {
        self.0.checked_add(1).map(Seq).ok_or_else(|| {
            AxError::failure(AxCode::InvalidArgs, "advance seq", self.0.to_string())
                .with_recovery("sequence space exhausted; this ledger cannot grow further")
        })
    }
}

/// UTC milliseconds as an integer (determinism rule 6). Always a
/// parameter, never sampled inside kernel or memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimeMs(u64);

impl TimeMs {
    pub fn new(value: u64) -> Self {
        TimeMs(value)
    }

    pub fn value(&self) -> u64 {
        self.0
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
    #[test]
    fn seq_and_time_are_checked_integers() {
        assert_eq!(Seq::FIRST.value(), 0);
        assert_eq!(Seq::FIRST.next().unwrap().value(), 1);
        assert!(Seq::new(u64::MAX).next().is_err());
        assert_eq!(TimeMs::new(1234).value(), 1234);
    }

    #[test]
    fn run_id_parses_and_city_is_nil() {
        assert_eq!(
            RunId::CITY.to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
        let run = RunId::parse("0198f6a2-7c4a-7bbb-9d1e-000000000001").unwrap();
        assert_eq!(run.to_string(), "0198f6a2-7c4a-7bbb-9d1e-000000000001");
        assert!(RunId::parse("not-a-uuid").is_err());
    }
}
