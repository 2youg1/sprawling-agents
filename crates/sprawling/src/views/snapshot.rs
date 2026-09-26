// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The views as the bytes a snapshot holds (sprawling-SPEC 8-91).

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash};

use super::Views;

/// Changed whenever a fold rule or a field of `Views` changes within one
/// version of this binary. A new version discards every snapshot anyway,
/// because `views_fold_version` hashes the version in with this.
const VIEWS_FOLD_RULES: &str = "views-fold-1";

/// The `fold_version` a views snapshot is cut and accepted under.
pub(crate) fn views_fold_version() -> u32 {
    let rules = [env!("CARGO_PKG_VERSION"), VIEWS_FOLD_RULES].join("\n");
    let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
    u32::from_le_bytes([a, b, c, d])
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
