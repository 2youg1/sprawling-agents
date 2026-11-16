// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash};

const LABEL_DIGITS: usize = 16;

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

#[must_use]
pub fn fixture_label(digest: &B3Hash) -> String {
    digest.to_string().chars().take(LABEL_DIGITS).collect()
}
