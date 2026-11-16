// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kernel::event::record::DocumentWritten;
use kernel::{Address, AxError, B3Hash, EventRecord, Seq, TimeMs};
use wire::{DocumentState, DocumentVersion, VersionSource};

use super::holding::Views;
use super::prepared::Prepared;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocumentSaves(BTreeMap<Address, Vec<Save>>);

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Save {
    seq: Seq,
    at: TimeMs,
    baseline: B3Hash,
    version: B3Hash,
    bytes: u64,
}

impl DocumentSaves {

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

pub struct VersionsAsk {
    city_root: PathBuf,
    at: Address,
    saves: Vec<Save>,
}

impl Views {

    pub(in crate::views) fn versions_ask(&self, at: &Address) -> Prepared {
        Prepared::Versions(VersionsAsk {
            city_root: self.city_root.clone(),
            at: at.clone(),
            saves: self.saves.0.get(at).cloned().unwrap_or_default(),
        })
    }
}

impl VersionsAsk {

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
        versions.extend(shipped(&self.city_root, &self.at));
        wire::Answer::Versions(Box::new(wire::VersionsAnswer {
            at: self.at,
            versions,
            more,
        }))
    }
}

fn shipped(city_root: &Path, at: &Address) -> Option<DocumentVersion> {
    let Some(city::PromptDocument::Shipped(doc)) = city::PromptDocument::of(at) else {
        return None;
    };
    let text = doc.text().as_bytes();
    Some(DocumentVersion {
        version: B3Hash::digest(text),
        bytes: u64::try_from(text.len()).ok(),
        kept: super::document::keep(city_root, text).is_ok(),
        source: VersionSource::Shipped,
    })
}

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
