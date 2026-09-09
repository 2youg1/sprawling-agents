// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Bundle manifests: what an export carries.

use std::path::Path;

use crate::error::MemoryError;

/// What a bundle claims to contain. Checked on restore, so a truncated
/// or half-copied bundle is refused rather than restored quietly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub(crate) records: u64,
    pub(crate) head: String,
    pub(crate) cas_objects: u64,
    pub(crate) files: u64,
}

impl Manifest {
    /// How many ledger records the bundle holds.
    #[must_use]
    pub fn records(&self) -> u64 {
        self.records
    }

    /// The chain hash after the last record: the one value that proves
    /// two ledgers are the same history and not merely the same length.
    #[must_use]
    pub fn head(&self) -> &str {
        &self.head
    }

    #[must_use]
    pub fn cas_objects(&self) -> u64 {
        self.cas_objects
    }

    /// City files, excluding everything under the reserved prefix.
    #[must_use]
    pub fn files(&self) -> u64 {
        self.files
    }

    pub(crate) fn to_json(&self) -> String {
        let mut map = serde_json::Map::new();
        map.insert("records".to_owned(), self.records.into());
        map.insert(
            "head".to_owned(),
            serde_json::Value::String(self.head.clone()),
        );
        map.insert("cas_objects".to_owned(), self.cas_objects.into());
        map.insert("files".to_owned(), self.files.into());
        serde_json::Value::Object(map).to_string()
    }

    pub(crate) fn from_json(bytes: &[u8], at: &Path) -> Result<Manifest, MemoryError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|err| MemoryError::Bundle {
                op: "read",
                detail: format!("{}: {err}", at.display()),
            })?;
        let number = |key: &str| -> Result<u64, MemoryError> {
            value
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| MemoryError::Bundle {
                    op: "read",
                    detail: format!("{} has no {key}", at.display()),
                })
        };
        Ok(Manifest {
            records: number("records")?,
            head: value
                .get("head")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| MemoryError::Bundle {
                    op: "read",
                    detail: format!("{} has no head", at.display()),
                })?
                .to_owned(),
            cas_objects: number("cas_objects")?,
            files: number("files")?,
        })
    }
}

/// The name of the file that states what a bundle holds.
pub const MANIFEST: &str = "MANIFEST.json";

pub(crate) const LEDGER: &str = "ledger";
pub(crate) const CAS: &str = "cas";
pub(crate) const CITY: &str = "city";
pub(crate) const RESERVED: &str = ".sprawling";

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::super::export::Bundle;
    use super::super::fixture::city_with;
    use super::*;

    #[test]
    fn the_manifest_is_byte_stable_across_exports() {
        let home = tempfile::tempdir().unwrap();
        city_with(2, home.path());
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        Bundle::export(home.path(), first.path()).unwrap();
        Bundle::export(home.path(), second.path()).unwrap();
        assert_eq!(
            std::fs::read(first.path().join(MANIFEST)).unwrap(),
            std::fs::read(second.path().join(MANIFEST)).unwrap()
        );
        assert_eq!(
            Bundle::read_manifest(first.path()).unwrap(),
            Bundle::read_manifest(second.path()).unwrap()
        );
    }
}
