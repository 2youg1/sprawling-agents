// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The browsers paired at this machine's door, kept in the city's
//! reserved subtree (`kernel::layout::CityLayout::browsers`,
//! `crates/sprawling/spec/Serving.lean`): public keys, labels and times,
//! never a secret.
//!
//! The rules of pairing are `wire::reception::pairing`'s; this reads the
//! table once when the city opens and writes the whole of it back after
//! every change, through a file beside it renamed into place.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{AxCode, AxError, TimeMs};
use serde::{Deserialize, Serialize};

/// The file's shape: one `[[browser]]` table per paired browser.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Table {
    #[serde(default, rename = "browser")]
    browsers: Vec<Row>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Row {
    id: String,
    label: String,
    public_key: String,
    paired_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_seen: Option<u64>,
}

/// Every browser paired before; none when the city never paired one.
///
/// # Errors
/// The file exists and cannot be read, or a line in it holds no key.
pub(crate) fn load(path: &Path) -> Result<Vec<wire::PairedBrowser>, AxError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(unreadable(path, &source.to_string())),
    };
    let table: Table = toml::from_str(&text).map_err(|err| unreadable(path, &err.to_string()))?;
    table
        .browsers
        .into_iter()
        .map(|row| {
            let key = wire::DeviceKey::parse(&row.public_key)
                .ok_or_else(|| unreadable(path, &format!("browser {} holds no key", row.id)))?;
            Ok(wire::PairedBrowser {
                id: key.id(),
                label: row.label,
                key,
                paired_at: TimeMs::new(row.paired_at),
                last_seen: row.last_seen.map(TimeMs::new),
            })
        })
        .collect()
}

/// Where the door keeps its table: the whole of it, written to `path`.
pub(crate) fn keeping(path: PathBuf) -> wire::KeepBrowsers {
    Arc::new(move |browsers: &[wire::PairedBrowser]| {
        let table = Table {
            browsers: browsers
                .iter()
                .map(|browser| Row {
                    id: browser.id.as_str().to_owned(),
                    label: browser.label.clone(),
                    public_key: browser.key.hex(),
                    paired_at: browser.paired_at.value(),
                    last_seen: browser.last_seen.map(|seen| seen.value()),
                })
                .collect(),
        };
        let text = toml::to_string(&table).map_err(|err| unwritten(&path, &err.to_string()))?;
        let staged = path.with_extension("toml.staged");
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|err| unwritten(dir, &err.to_string()))?;
        }
        std::fs::write(&staged, text).map_err(|err| unwritten(&staged, &err.to_string()))?;
        std::fs::rename(&staged, &path).map_err(|err| unwritten(&path, &err.to_string()))
    })
}

fn unreadable(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "read the paired browsers",
        format!("{}: {why}", path.display()),
    )
    .with_recovery("move the file aside to pair every browser again; it holds public keys only")
}

fn unwritten(path: &Path, why: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "keep the paired browsers",
        format!("{}: {why}", path.display()),
    )
    .with_recovery("check the city directory is writable, then pair again")
}
