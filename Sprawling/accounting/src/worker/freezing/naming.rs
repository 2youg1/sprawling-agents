// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxCode, AxError, B3Hash};

use super::{Assembled, NEWLINE, addressed, prompt_segment};

pub(super) struct Frozen {
    naming: city::Naming,
    pub(super) version: B3Hash,
}

pub(super) fn session_naming(
    city_root: &Path,
    cas: &mut storage::Cas,
    room: &Address,
) -> Result<Frozen, AxError> {
    if let Some(version) = city::own_layer(city_root, room)?.naming() {
        let bytes = cas.get(&version).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the names this session froze",
                format!("{}: {version}: {err}", room.as_str()),
            )
            .with_recovery(
                "start a new session here (`/new`), which freezes the names as they are now",
            )
        })?;
        return Ok(Frozen {
            naming: city::Naming::from_bytes(&bytes)?,
            version,
        });
    }
    let naming = city::Naming::read(city_root)?;
    let version = cas
        .put(&naming.to_bytes()?)
        .map_err(storage::StorageError::into_ax)?;
    city::freeze_naming(city_root, room, version)?;
    Ok(Frozen { naming, version })
}

impl Frozen {
    pub(super) fn city_slot(
        &self,
        city_root: &Path,
        building: &Address,
    ) -> Result<Assembled, AxError> {
        let path = city::city_prompt_path(city_root, building)?;
        let mut slot = prompt_segment(&path, addressed(city_root, &path)?)?;
        if let Some(context) = self.naming.context() {
            if !slot.bytes.is_empty() {
                slot.bytes.push(NEWLINE);
                slot.bytes.push(NEWLINE);
            }
            slot.bytes.extend_from_slice(context.as_bytes());
        }
        Ok(slot)
    }

    pub(super) fn called(&self, addr: &Address) -> Vec<u8> {
        format!("Your name: {}\n\n", self.naming.called(addr)).into_bytes()
    }
}
