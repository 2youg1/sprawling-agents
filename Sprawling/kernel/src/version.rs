// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The version of one subject: a counter that starts at
//! [`Version::FIRST`] and only moves forward, checked. A signal records
//! the version of the room it was sent to (`room_version`).
//! The part `crates/kernel/spec/Version.lean` specifies this module.
//!
//! A file a run edits is guarded by the run's own read, not by a
//! version (`crates/runtime/Spec.lean` §8-64), so no write
//! judges freshness here.

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

/// Monotonic per-subject version; starts at [`Version::FIRST`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(u64);

impl Version {
    /// The first visible version of any subject.
    pub const FIRST: Version = Version(1);

    pub fn new(value: u64) -> Self {
        Version(value)
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn next(self) -> Result<Version, AxError> {
        self.0.checked_add(1).map(Version).ok_or_else(|| {
            AxError::failure(AxCode::InvalidArgs, "advance version", self.0.to_string())
                .with_recovery("version space exhausted for this subject")
        })
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
    fn first_version_is_one_and_advances_checked() {
        assert_eq!(Version::FIRST.value(), 1);
        assert_eq!(Version::FIRST.next().unwrap().value(), 2);
        assert!(Version::new(u64::MAX).next().is_err());
    }

    #[test]
    fn serde_is_transparent_u64() {
        assert_eq!(serde_json::to_string(&Version::new(3)).unwrap(), "3");
        let v: Version = serde_json::from_str("9").unwrap();
        assert_eq!(v, Version::new(9));
    }
}
