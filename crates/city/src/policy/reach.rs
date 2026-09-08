// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a building's residents may write inside their prefixes.
//!
//! One line of `BUILDING.md` decides it, and the value is exhaustive
//! rather than a flag: "every file here" and "the Markdown documents
//! here" are two policies, and City Hall exists because the second one
//! had to be sayable.

use kernel::{AxCode, AxError};

/// What a building's residents may write inside their prefixes.
///
/// Exhaustive rather than a flag on the prefix list: "every file here"
/// and "the Markdown documents here" are two policies, and City Hall
/// exists because the second one had to be sayable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainReach {
    Everything,
    Documents,
}

impl DomainReach {
    /// Reads the `write:` line. Absent is `Everything`: every building
    /// written before this line existed writes what it always wrote.
    ///
    /// # Errors
    /// Refuses a value that is neither spelling, for the reason
    /// `confidential:` does — a permission setting that reads as a typo
    /// must not resolve to the permissive side.
    pub(super) fn parse(value: &str) -> Result<DomainReach, AxError> {
        match value {
            "everything" => Ok(DomainReach::Everything),
            "documents" => Ok(DomainReach::Documents),
            other => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "evaluate a building's rules",
                format!("`write: {other}` is neither everything nor documents"),
            )
            .with_recovery("write `write: everything` or `write: documents`")),
        }
    }
}
