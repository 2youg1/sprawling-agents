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
pub struct RunId(#[serde(deserialize_with = "read_run_id")] uuid::Uuid);

impl RunId {
    /// City-level records (genesis, tail truncation) carry the nil id;
    /// real runs use uuid v7, whose timestamp bits never collide with nil.
    pub const CITY: RunId = RunId(uuid::Uuid::nil());

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        RunId(uuid::Uuid::from_bytes(bytes))
    }

    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let reject = || {
            let err = AxError::failure(AxCode::InvalidArgs, "parse run id", raw).with_recovery(
                "use the hyphenated lower-case uuid this city writes, as \
                 `0198f6a2-7c4a-7bbb-9d1e-000000000001`",
            );
            Err(err)
        };
        let parsed = match uuid::Uuid::parse_str(raw) {
            Ok(parsed) => parsed,
            Err(_) => return reject(),
        };
        // Canonical echo, as `kernel::locator` does: the value's own
        // spelling is the only input accepted, which closes bare hex,
        // braces, the `urn:uuid:` prefix and upper case in one rule.
        if parsed.hyphenated().to_string() != raw {
            return reject();
        }
        Ok(RunId(parsed))
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

/// Reads a run id through [`RunId::parse`], so an identity that arrives
/// in a frame or a ledger line cannot be read in a spelling the
/// constructor refuses. The derived `Deserialize` would otherwise ask
/// `uuid::Uuid` directly, which takes four spellings to this type's one.
fn read_run_id<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<uuid::Uuid, D::Error> {
    let raw = String::deserialize(deserializer)?;
    RunId::parse(&raw)
        .map(|run| run.0)
        .map_err(serde::de::Error::custom)
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.hyphenated().fmt(f)
    }
}

/// Event sequence number; contiguous from [`Seq::FIRST`], checked arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    fn a_run_id_is_read_in_the_one_spelling_the_city_writes() {
        assert_eq!(
            RunId::CITY.to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
        let run = RunId::parse("0198f6a2-7c4a-7bbb-9d1e-000000000001").unwrap();
        assert_eq!(run.to_string(), "0198f6a2-7c4a-7bbb-9d1e-000000000001");
        for refused in [
            "not-a-uuid",
            "0198f6a27c4a7bbb9d1e000000000001",       // bare hex
            "{0198f6a2-7c4a-7bbb-9d1e-000000000001}", // braced
            "urn:uuid:0198f6a2-7c4a-7bbb-9d1e-000000000001", // urn
            "0198F6A2-7C4A-7BBB-9D1E-000000000001",   // upper case
        ] {
            assert!(RunId::parse(refused).is_err(), "{refused}");
        }
    }

    /// What arrives in a frame or a ledger line is read by the same
    /// constructor the city writes with, so a spelling the city would
    /// never write cannot arrive and be believed.
    #[test]
    fn serde_reads_a_run_id_through_its_sole_constructor() {
        let written = "\"0198f6a2-7c4a-7bbb-9d1e-000000000001\"";
        let run: RunId = serde_json::from_str(written).unwrap();
        assert_eq!(serde_json::to_string(&run).unwrap(), written);
        for refused in [
            "\"0198f6a27c4a7bbb9d1e000000000001\"",
            "\"{0198f6a2-7c4a-7bbb-9d1e-000000000001}\"",
            "\"urn:uuid:0198f6a2-7c4a-7bbb-9d1e-000000000001\"",
        ] {
            assert!(serde_json::from_str::<RunId>(refused).is_err(), "{refused}");
        }
    }
}
