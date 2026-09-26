// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The views as the bytes a snapshot holds (sprawling-SPEC 8-91).

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash, Seq};

use super::Views;

/// Changed whenever a fold rule or the encoding of `Views` changes within
/// one version of this binary; a new version discards every snapshot
/// anyway, because `views_fold_version` hashes the version in with this.
/// The suffix is the digest of a fixed fixture's encoding, which the
/// tests beside this file hold, so the encoding cannot move alone.
const VIEWS_FOLD_RULES: &str = "views-fold-a41c5ca5671d61ce";

/// The `fold_version` a views snapshot is cut and accepted under.
pub(crate) fn views_fold_version() -> u32 {
    let rules = [env!("CARGO_PKG_VERSION"), VIEWS_FOLD_RULES].join("\n");
    let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// The index a new or decoded `Views` starts with: empty, refreshed by
/// the first question that reads the ledger.
pub(super) fn fresh_index() -> std::sync::Arc<std::sync::Mutex<memory::LedgerIndex>> {
    std::sync::Arc::new(std::sync::Mutex::new(memory::LedgerIndex::empty()))
}

impl Views {
    /// The folded state, without the fields `Views::new` rebuilds.
    ///
    /// # Errors
    /// `StorageFatal` when a field refuses to serialise.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, AxError> {
        postcard::to_allocvec(self).map_err(|fault| {
            AxError::failure(AxCode::StorageFatal, "encode the views", fault.to_string())
                .with_recovery(
                    "restart the server; the views fold from the ledger without a snapshot",
                )
        })
    }

    /// The last line these views folded, read through their index, which
    /// is where a snapshot of them is cut. `None` before genesis is
    /// folded.
    ///
    /// # Errors
    /// `StorageFatal` when the index lock is poisoned, and the ledger
    /// read's own failure.
    pub(crate) fn last_folded_line(
        &self,
        ledger_dir: &Path,
    ) -> Result<Option<(Seq, Vec<u8>)>, AxError> {
        let Some(seq) = self.next_unfolded.value().checked_sub(1).map(Seq::new) else {
            return Ok(None);
        };
        let index = self.index.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the last folded line",
                "the ledger index lock is poisoned",
            )
            .with_recovery("restart the server; the index is rebuilt from the ledger")
        })?;
        index
            .reader(ledger_dir)
            .line_at(seq)
            .map(|line| Some((seq, line)))
            .map_err(memory::MemoryError::into_ax)
    }

    /// The views `encode` wrote, served from `city_root`.
    ///
    /// # Errors
    /// `CasCorrupt` when the bytes are not an encoding of this build's
    /// `Views`; the caller folds from genesis instead.
    pub(crate) fn decode(city_root: &Path, bytes: &[u8]) -> Result<Views, AxError> {
        let folded: Views = postcard::from_bytes(bytes).map_err(|fault| {
            AxError::failure(AxCode::CasCorrupt, "decode the views", fault.to_string())
                .with_recovery("none needed: the views fold from genesis and a new snapshot is cut")
        })?;
        let fresh = Views::new(city_root);
        Ok(Views {
            city_root: fresh.city_root,
            index: fresh.index,
            machine: fresh.machine,
            vault: fresh.vault,
            ..folded
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
mod tests;
