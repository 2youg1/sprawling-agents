// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes a reading was taken over, named by their digest
//! (citysim D8, `tools/citysim/spec/Bench.lean` §8-6).
//!
//! Shape: value. Both bench families read it: `bench` pins the digest of
//! its registered fixture, and `bench_startup` prints the digest of each
//! fixture city beside its first-byte reading. Two readings are
//! comparable only when these digests are equal.

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash};

/// How many hex digits a reading line names its fixture by.
const LABEL_DIGITS: usize = 16;

/// The digest of the ledger in `ledger_dir`: each segment's bytes are
/// digested in the order `storage::ledger_segments_at` gives, and the
/// 32-byte digests, joined in that order, are digested once more.
///
/// One segment's bytes are held at a time, so a ledger of any length is
/// read in the memory of its largest segment.
///
/// # Errors
/// `StorageFatal` when the segments cannot be listed or one cannot be
/// read.
pub fn ledger_digest(ledger_dir: &Path) -> Result<B3Hash, AxError> {
    let mut joined = Vec::new();
    for segment in
        storage::ledger_segments_at(ledger_dir).map_err(storage::StorageError::into_ax)?
    {
        let bytes = std::fs::read(&segment).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "read a ledger segment to digest it",
                format!("{}: {err}", segment.display()),
            )
            .with_recovery("check the ledger directory is readable, then take the reading again")
        })?;
        joined.extend_from_slice(B3Hash::digest(&bytes).as_bytes());
    }
    Ok(B3Hash::digest(&joined))
}

/// The label a reading line and a report name a fixture by: the first
/// sixteen hex digits of its digest. The whole digest is what a pin
/// holds; the label is what a person compares by eye.
#[must_use]
pub fn fixture_label(digest: &B3Hash) -> String {
    digest.to_string().chars().take(LABEL_DIGITS).collect()
}
