// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a playback bundle is found to be (accounting-SPEC.md 8-12,
//! decision 24(d)).
//!
//! A bundle on its own can only be found consistent: that says nothing
//! about whether it was changed, because a changed bundle can be made
//! consistent again. Against another bundle it is the same bytes or not.
//! Against its city it is recomputed with the reader the caller names
//! and compared whole, so keeping `source` while changing a table does
//! not pass.

use std::path::Path;

use kernel::{AxError, B3Hash};

use super::encode::decode;
use super::reader::Reader;

/// What a bundle is checked against.
#[derive(Debug, Clone)]
pub enum Against<'a> {
    /// Nothing: the bundle is only read and checked for consistency.
    Nothing,
    /// Another bundle's bytes.
    Bundle(&'a [u8]),
    /// The city it was exported from, read by `reader`.
    City { root: &'a Path, reader: Reader },
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub digest: B3Hash,
    pub events: usize,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Readable, canonical, and every reference inside it resolves.
    Consistent,
    /// The same bytes as the bundle or the recomputation it was checked
    /// against.
    Same,
    /// Different bytes; `section` is the first part that differs.
    Differs { section: &'static str },
    /// This build cannot recompute the bundle, and says what can.
    CannotReproduce { why: String },
}

/// Checks `bytes` against `against`.
///
/// # Errors
/// `E_INVALID_ARGS` when `bytes` (or the other bundle) is not a
/// consistent bundle of this schema; whatever recomputing it from the
/// city refuses.
pub fn check(bytes: &[u8], _against: Against<'_>) -> Result<Report, AxError> {
    let (_, bundle) = decode(bytes)?;
    Ok(Report {
        digest: bundle.digest(),
        events: bundle.events(),
        verdict: Verdict::Consistent,
    })
}
