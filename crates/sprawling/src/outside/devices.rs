// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The device table on disk: the devices this city paired, kept across
//! restarts at `kernel::layout::CityLayout::devices` (sprawling-SPEC.md
//! 8-139).
//!
//! The file is TOML, one `[[device]]` table per device, and is written
//! whole: a new file beside the old one, then renamed over it, so a
//! crash leaves either the table before the change or the table after
//! it. The spellings of an id, a key and an authority are the remote
//! door's own (`remote_access::door`); this module only arranges them.
//!
//! The point a reader most often gets wrong: an absent file is an empty
//! table, because a city that never paired a device has never written
//! one; an unreadable file is refused, because guessing a table would be
//! guessing who may enter.

use std::path::Path;

use kernel::{AxCode, AxError};
use remote_access::door::{Authority, Device, DeviceId, DeviceKey, DeviceName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
struct Table {
    #[serde(default)]
    device: Vec<Row>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Row {
    id: String,
    name: String,
    authority: String,
    key: String,
}

/// The paired devices the table at `path` holds.
///
/// # Errors
/// `E_STORAGE_FATAL`, naming the file, when it cannot be read or does
/// not read as a device table.
pub(super) fn read(path: &Path) -> Result<Vec<Device>, AxError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(fatal("read the device table", path, &source.to_string())),
    };
    let table: Table = toml::from_str(&text)
        .map_err(|source| fatal("read the device table", path, &source.to_string()))?;
    table
        .device
        .iter()
        .map(|row| {
            Ok(Device {
                id: DeviceId::read(&row.id)?,
                name: DeviceName::parse(&row.name)?,
                authority: Authority::from_word(&row.authority)?,
                key: DeviceKey::read(&row.key)?,
            })
        })
        .collect::<Result<Vec<Device>, AxError>>()
        .map_err(|refused| fatal("read the device table", path, refused.subject()))
}

/// Replaces the table at `path` with `devices`.
///
/// # Errors
/// `E_STORAGE_FATAL`, naming the file, when the directory cannot be made
/// or the file cannot be written or renamed into place.
pub(super) fn write(path: &Path, devices: &[Device]) -> Result<(), AxError> {
    let table = Table {
        device: devices
            .iter()
            .map(|device| Row {
                id: device.id.text(),
                name: device.name.as_str().to_owned(),
                authority: device.authority.word().to_owned(),
                key: device.key.text(),
            })
            .collect(),
    };
    let text = toml::to_string(&table)
        .map_err(|source| fatal("write the device table", path, &source.to_string()))?;
    let beside = path.with_extension("toml.new");
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&beside, text))
        .and_then(|()| std::fs::rename(&beside, path));
    written.map_err(|source| fatal("write the device table", path, &source.to_string()))
}

fn fatal(action: &str, path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        action,
        format!("{}: {why}", path.display()),
    )
    .with_recovery(
        "check that the city directory is readable and writable; a table changed by hand \
         can be deleted, and every device paired again",
    )
}
