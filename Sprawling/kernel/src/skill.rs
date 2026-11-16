// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The canonical manifest of one skill package
//! (`crates/kernel/spec/Skill.lean`). A manifest names every package item
//! and each file's CAS digest; the package digest is the digest of these
//! bytes. It carries no file bytes itself and performs no I/O.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{AxCode, AxError, B3Hash};

/// The manifest grammar this build reads and writes.
pub const SKILL_MANIFEST_VERSION: u16 = 1;

/// Whether an item is a directory or a file, derived from the item rather
/// than stored a second time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillItemKind {
    Directory,
    File,
}

/// One package item. The path is relative to the package and always uses
/// `/`, on every platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillItem {
    Directory { path: String },
    File { path: String, digest: B3Hash },
}

impl SkillItem {
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            SkillItem::Directory { path } | SkillItem::File { path, .. } => path,
        }
    }

    #[must_use]
    pub fn kind(&self) -> SkillItemKind {
        match self {
            SkillItem::Directory { .. } => SkillItemKind::Directory,
            SkillItem::File { .. } => SkillItemKind::File,
        }
    }

    #[must_use]
    pub fn digest(&self) -> Option<&B3Hash> {
        match self {
            SkillItem::Directory { .. } => None,
            SkillItem::File { digest, .. } => Some(digest),
        }
    }
}

/// The validated, canonically ordered items of one package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillManifest {
    version: u16,
    items: Vec<SkillItem>,
}

impl SkillManifest {
    /// Validates and sorts the items. Exactly one file named
    /// `kernel::layout::SKILL_FILE` must be present.
    pub fn new(mut items: Vec<SkillItem>) -> Result<Self, AxError> {
        items.sort_by(|left, right| left.path().cmp(right.path()));
        validate(&items).map_err(invalid_source)?;
        Ok(Self {
            version: SKILL_MANIFEST_VERSION,
            items,
        })
    }

    /// The manifest's canonical JSON bytes. Item order was fixed at
    /// construction and field order is the declarations above.
    pub fn encode(&self) -> Result<Vec<u8>, AxError> {
        serde_json::to_vec(self).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "encode a skill manifest",
                err.to_string(),
            )
            .with_recovery("construct the manifest from plain relative package items")
        })
    }

    /// Reads and revalidates a manifest. Any malformed or non-current
    /// object is corruption, never an empty package.
    pub fn decode(bytes: &[u8]) -> Result<Self, AxError> {
        let manifest: SkillManifest = serde_json::from_slice(bytes).map_err(corrupt)?;
        if manifest.version != SKILL_MANIFEST_VERSION {
            return Err(corrupt(format!(
                "manifest version {} is not {SKILL_MANIFEST_VERSION}",
                manifest.version
            )));
        }
        validate(&manifest.items).map_err(corrupt)?;
        Ok(manifest)
    }

    #[must_use]
    pub fn items(&self) -> &[SkillItem] {
        &self.items
    }

    /// The digest of a file at exactly this relative path, never of a
    /// directory or a prefix match.
    #[must_use]
    pub fn file(&self, path: &str) -> Option<&B3Hash> {
        self.items.iter().find_map(|item| match item {
            SkillItem::File { path: held, digest } if held == path => Some(digest),
            SkillItem::File { .. } | SkillItem::Directory { .. } => None,
        })
    }

    /// The package's entry document, whose one name lives in `layout`.
    #[must_use]
    pub fn document(&self) -> Option<&B3Hash> {
        self.file(crate::layout::SKILL_FILE)
    }
}

fn validate(items: &[SkillItem]) -> Result<(), String> {
    let mut paths = BTreeSet::new();
    for item in items {
        let path = item.path();
        if !plain_package_path(path) {
            return Err(format!("`{path}` is not a plain relative package path"));
        }
        if !paths.insert(path) {
            return Err(format!("`{path}` occurs more than once"));
        }
    }
    let documents = items
        .iter()
        .filter(|item| {
            item.path() == crate::layout::SKILL_FILE && item.kind() == SkillItemKind::File
        })
        .count();
    if documents != 1 {
        return Err(format!(
            "the package needs exactly one {} file",
            crate::layout::SKILL_FILE
        ));
    }
    Ok(())
}

/// Whether `path` is one canonical relative path inside a skill package.
/// Manifest construction and package reads share this grammar.
#[must_use]
pub fn plain_package_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && !matches!(segment, "." | "..")
                && !segment.contains(['\\', ':', '\0'])
        })
}

fn invalid_source(why: String) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "construct a skill manifest", why).with_recovery(
        "use unique plain relative paths and exactly one SKILL.md file, then install again",
    )
}

fn corrupt(why: impl std::fmt::Display) -> AxError {
    AxError::failure(AxCode::CasCorrupt, "read a skill manifest", why.to_string())
        .with_recovery("reinstall the skill through the city's install door, then read it again")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn file(path: &str, byte: u8) -> SkillItem {
        SkillItem::File {
            path: path.to_owned(),
            digest: B3Hash::digest(&[byte]),
        }
    }

    #[test]
    fn a_manifest_round_trips_canonically_and_looks_up_only_named_files() {
        let manifest = SkillManifest::new(vec![
            file("scripts/check.sh", 2),
            SkillItem::Directory {
                path: "scripts".to_owned(),
            },
            file("SKILL.md", 1),
        ])
        .unwrap();
        let bytes = manifest.encode().unwrap();
        assert_eq!(
            B3Hash::digest(&bytes),
            B3Hash::digest(&manifest.encode().unwrap())
        );
        let read = SkillManifest::decode(&bytes).unwrap();
        assert_eq!(read, manifest);
        assert_eq!(read.file("scripts/check.sh"), Some(&B3Hash::digest(&[2])));
        assert_eq!(read.document(), Some(&B3Hash::digest(&[1])));
        assert_eq!(read.file("scripts"), None);
        assert_eq!(read.file("other"), None);
    }

    #[test]
    fn invalid_paths_duplicates_and_missing_documents_are_refused() {
        for path in ["", ".", "..", "a//b", "a\\b", "a:b", "a/\0b"] {
            let error = SkillManifest::new(vec![file(path, 1), file("SKILL.md", 2)]);
            assert!(error.is_err(), "{path:?} was accepted");
        }
        assert!(SkillManifest::new(vec![file("SKILL.md", 1), file("SKILL.md", 1)]).is_err());
        assert!(SkillManifest::new(vec![file("other.md", 1)]).is_err());
        assert!(
            SkillManifest::new(vec![SkillItem::Directory {
                path: "SKILL.md".to_owned(),
            },])
            .is_err()
        );
    }

    #[test]
    fn malformed_and_unknown_version_objects_are_corruption() {
        assert_eq!(
            SkillManifest::decode(b"not json").unwrap_err().code(),
            &AxCode::CasCorrupt
        );
        let manifest = SkillManifest::new(vec![file("SKILL.md", 1)]).unwrap();
        let mut value = serde_json::to_value(manifest).unwrap();
        value["version"] = serde_json::Value::from(99);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            SkillManifest::decode(&bytes).unwrap_err().code(),
            &AxCode::CasCorrupt
        );
    }
}
