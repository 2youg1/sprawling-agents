// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The versions of one document, newest first (`crates/accounting/spec/Views/Document.lean` §8-21,
//! `crates/wire/spec/Answer/DocumentVersions.lean` §8-83).
//!
//! The saves are folded from `document_written`, so a page asking about a
//! document never walks the ledger. The version on disk is read when the
//! question is asked, after the view lock is released, through the same
//! door `Document` reads by - so it lands in the content store as every
//! version the city reads does.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kernel::event::record::DocumentWritten;
use kernel::{Address, AxError, B3Hash, EventRecord, Seq, TimeMs};
use wire::{DocumentState, DocumentVersion, VersionSource};

use super::holding::Views;
use super::prepared::Prepared;

/// Every save of every document the history recorded, oldest first.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocumentSaves(BTreeMap<Address, Vec<Save>>);

/// One `document_written` line, as the versions answer reads it.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Save {
    seq: Seq,
    at: TimeMs,
    baseline: B3Hash,
    version: B3Hash,
    bytes: u64,
}

impl DocumentSaves {
    /// Folds one `document_written` line.
    ///
    /// # Errors
    /// A line whose payload does not read as a save.
    pub(super) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        let written = record.data().read::<DocumentWritten>()?;
        self.0.entry(written.at).or_default().push(Save {
            seq: record.seq(),
            at: record.t(),
            baseline: written.baseline,
            version: written.version,
            bytes: written.bytes,
        });
        Ok(())
    }
}

/// The question, with the saves copied out of the views and the disk
/// and the store still to read.
pub struct VersionsAsk {
    city_root: PathBuf,
    at: Address,
    saves: Vec<Save>,
}

impl Views {
    /// The saves of `at` are copied now; the disk is read once the views
    /// are released.
    pub(in crate::views) fn versions_ask(&self, at: &Address) -> Prepared {
        Prepared::Versions(VersionsAsk {
            city_root: self.city_root.clone(),
            at: at.clone(),
            saves: self.saves.0.get(at).cloned().unwrap_or_default(),
        })
    }
}

impl VersionsAsk {
    /// The versions, newest first, at most [`wire::VERSIONS_MAX`].
    pub(in crate::views) fn answer(self) -> wire::Answer {
        let on_disk = on_disk(&self.city_root, self.at.clone());
        let store =
            storage::Cas::open(&kernel::layout::CityLayout::new(&self.city_root).cas()).ok();
        let kept = |version: &B3Hash| store.as_ref().is_some_and(|cas| cas.contains(version));
        let size = |version: &B3Hash| store.as_ref().and_then(|cas| cas.size(version).ok());
        let mut versions = Vec::new();
        let newest_saved = self.saves.last().map(|save| save.version);
        if let Some(version) = on_disk.filter(|disk| Some(*disk) != newest_saved) {
            versions.push(DocumentVersion {
                version,
                bytes: size(&version),
                kept: kept(&version),
                source: VersionSource::OnDisk,
            });
        }
        let mut older = self.saves.iter().rev().skip(1).map(Some).chain([None]);
        for save in self.saves.iter().rev() {
            versions.push(DocumentVersion {
                version: save.version,
                bytes: Some(save.bytes),
                kept: kept(&save.version),
                source: VersionSource::Saved {
                    seq: save.seq,
                    at: save.at,
                },
            });
            let before = older.next().flatten();
            if before.is_none_or(|earlier| earlier.version != save.baseline) {
                versions.push(DocumentVersion {
                    version: save.baseline,
                    bytes: size(&save.baseline),
                    kept: kept(&save.baseline),
                    source: VersionSource::Before { seq: save.seq },
                });
            }
        }
        let more = versions.len() > wire::VERSIONS_MAX;
        versions.truncate(wire::VERSIONS_MAX);
        wire::Answer::Versions(Box::new(wire::VersionsAnswer {
            at: self.at,
            versions,
            more,
        }))
    }
}

/// The version on disk now, read through `Document`'s door so it is
/// kept as every version the city reads is; none when nothing readable
/// is there.
fn on_disk(city_root: &Path, at: Address) -> Option<B3Hash> {
    match super::document::document_answer(city_root, at).state {
        DocumentState::Empty { version, .. } => Some(version),
        DocumentState::Held(held) => Some(held.version),
        DocumentState::Missing | DocumentState::Unreadable { .. } => None,
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
