// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Putting one recycle-bin row back by the way back it carries
//! (sprawling-SPEC §8-107).

use crate::assembly::RunWorker;
use kernel::{AxCode, AxError, EventKind, Locator, Payload, Restoration};

impl RunWorker {
    /// Writes the discarded file back, then records that it came back.
    ///
    /// # Errors
    /// Refuses a way back this city cannot follow, and propagates the
    /// checkpoint that cannot find or write the bytes and the ledger
    /// that refuses the append.
    pub(in crate::assembly) fn restore_discard(
        &mut self,
        restoration: &Restoration,
    ) -> Result<(), AxError> {
        let (address, oid) = match restoration {
            Restoration::Tracked(Locator::File {
                address,
                oid,
                range: None,
            }) => (address, oid),
            Restoration::Tracked(
                whole @ (Locator::File { range: Some(_), .. } | Locator::Cas { .. }),
            ) => {
                return Err(refused(
                    whole.to_string(),
                    "restore the whole file: a way back that names a part of one, \
                     or no file, is not one the recycle bin writes",
                ));
            }
            Restoration::Interred(interred) => {
                return Err(refused(
                    interred.to_string(),
                    "fetching a row back from the content store is not wired yet; \
                     read its bytes from the content store by the locator it names",
                ));
            }
            Restoration::Rebuildable { reason } => {
                return Err(refused("rebuildable".to_owned(), reason.clone()));
            }
        };
        // Disk first, ledger second: the history never says a file came
        // back that the disk does not hold.
        storage::Checkpoint::open(&self.city_root)
            .and_then(|checkpoint| checkpoint.restore(address, oid))
            .map_err(storage::StorageError::into_ax)?;
        // The same shape as the `file_discarded` it closes, so one
        // reader folds both (§8-107).
        let restored = Payload::new(serde_json::Map::from_iter([
            (
                "paths".to_owned(),
                serde_json::Value::Array(vec![serde_json::Value::String(format!(
                    "file:{address}"
                ))]),
            ),
            (
                "restoration".to_owned(),
                serde_json::to_value(restoration).map_err(|err| {
                    AxError::failure(AxCode::InvalidArgs, "encode a restoration", err.to_string())
                        .with_recovery(
                            "report this against sprawling::assembly::commanding::restoring: \
                             a restoration is text only",
                        )
                })?,
            ),
        ]))?;
        self.record(EventKind::DiscardRestored, restored)
    }
}

fn refused(subject: String, recovery: impl Into<String>) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "restore a discarded file", subject)
        .with_recovery(recovery)
}
