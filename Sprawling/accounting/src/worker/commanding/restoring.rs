// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::worker::RunWorker;
use kernel::{AxCode, AxError, EventKind, Locator, Payload, Restoration};

impl RunWorker {

    pub(in crate::worker) fn restore_discard(
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

        storage::Checkpoint::open(&self.city_root)
            .and_then(|checkpoint| checkpoint.restore(address, oid))
            .map_err(storage::StorageError::into_ax)?;

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
                            "report this against crate::worker::commanding::restoring: \
                             a restoration is text only",
                        )
                })?,
            ),
        ]))?;
        self.record(EventKind::DiscardRestored, restored)
    }
}

impl RunWorker {

    pub(in crate::worker) fn take_back(
        &mut self,
        at: &kernel::Address,
        point: kernel::GitOid,
    ) -> Result<(), AxError> {
        let building = city::Building::of(at)?;
        if let Some((room, working)) = self.collaborating.rooms.worked_within(building.addr()) {
            return Err(AxError::failure(
                AxCode::Busy,
                "take a file back from a checkpoint",
                format!("{}: {working} is running in {}", at.as_str(), room.as_str()),
            )
            .with_recovery(
                "stop that run first, then take the file back; a file cannot change under \
                 the run writing beside it",
            ));
        }

        let restored = storage::Checkpoint::open(&self.city_root)
            .and_then(|checkpoint| checkpoint.take_back(at, &point))
            .map_err(storage::StorageError::into_ax)?;
        self.record(EventKind::FileRestored, Payload::of(&restored)?)
    }
}

fn refused(subject: String, recovery: impl Into<String>) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "restore a discarded file", subject)
        .with_recovery(recovery)
}
