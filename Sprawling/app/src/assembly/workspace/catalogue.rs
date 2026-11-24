// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

use super::disk::failure;
use super::{Catalogue, Registry};

const SCHEMA: u8 = 1;

impl Registry {
    pub(super) fn catalogue(&self) -> Result<&Catalogue, AxError> {
        self.table.as_ref().ok_or_else(|| {
            AxError::failure(AxCode::StorageFatal, "read the workspace registry", self.table_path().display().to_string())
                .with_recovery("open the workspace registry again and check that the city can read its workspace catalogue")
        })
    }

    pub(super) fn table_path(&self) -> PathBuf {
        self.city
            .join(kernel::RESERVED_PREFIX)
            .join("refrain-workspaces.json")
    }

    pub(super) fn read_catalogue(&self) -> Result<Catalogue, AxError> {
        let path = self.table_path();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(Catalogue {
                    schema: SCHEMA,
                    by_id: BTreeMap::new(),
                });
            }
            Err(err) => return Err(failure("read workspace catalogue", &path, &err)),
        };
        let table: Catalogue =
            serde_json::from_slice(&bytes).map_err(|err| broken(&path, err.to_string()))?;
        if table.schema != SCHEMA {
            return Err(broken(
                &path,
                format!("found catalogue schema {}, expected {SCHEMA}", table.schema),
            ));
        }
        Ok(table)
    }

    pub(super) fn commit(&mut self, next: Catalogue) -> Result<(), AxError> {
        let path = self.table_path();
        let bytes = serde_json::to_vec(&next).map_err(|err| broken(&path, err.to_string()))?;
        let target = storage::WriteTarget::at("write workspace catalogue", &path)
            .map_err(storage::StorageError::into_ax)?;
        if let Err(err) = target.replace(&bytes) {
            self.table = None;
            return Err(err.into_ax());
        }
        self.table = Some(next);
        Ok(())
    }
}

fn broken(path: &Path, reason: String) -> AxError {
    AxError::failure(AxCode::StorageFatal, "read the local workspace catalogue", format!("{}: {reason}", path.display()))
        .with_recovery("preserve the catalogue, then restore a readable backup or use a compatible RefRain version")
}
