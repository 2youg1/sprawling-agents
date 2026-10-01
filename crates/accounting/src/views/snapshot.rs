// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The views as the bytes a snapshot holds (sprawling-SPEC 8-91).

use std::path::Path;

use std::sync::{Arc, Mutex};

use crate::plan_view::PlanView;
use kernel::{AxCode, AxError, B3Hash, EventRecord};
use storage::LedgerIndex;

use super::Views;

pub mod start;

use start::SnapshotFold;

/// Changed whenever a fold rule or the encoding of `Views` changes within
/// one version of this binary; a new version discards every snapshot
/// anyway, because `views_fold_version` hashes the version in with this.
/// The suffix is the digest of a fixed fixture's encoding, which the
/// tests beside this file hold, so the encoding cannot move alone.
const VIEWS_FOLD_RULES: &str = "views-fold-18aa5cc292f8e281";

/// The `fold_version` a views snapshot is cut and accepted under.
pub(crate) fn views_fold_version() -> u32 {
    let rules = [env!("CARGO_PKG_VERSION"), VIEWS_FOLD_RULES].join("\n");
    let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// The index a new `Views` starts with: empty, until a fold hands over
/// the index it built or the first question that reads the ledger
/// refreshes it.
pub(super) fn fresh_index() -> std::sync::Arc<std::sync::Mutex<storage::LedgerIndex>> {
    std::sync::Arc::new(std::sync::Mutex::new(storage::LedgerIndex::empty()))
}

/// The ledger index as a snapshot holds it: the index itself, not the
/// lock the two copies of the views share it through. A lock a panic
/// poisoned is taken as it stands: the index is a projection that
/// `refresh` rebuilds on any doubt (storage-SPEC 8-4).
pub(super) fn encode_index<S: serde::Serializer>(
    index: &Arc<Mutex<LedgerIndex>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(
        &*index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        serializer,
    )
}

/// The index `encode_index` wrote, behind a lock of its own.
pub(super) fn decode_index<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Arc<Mutex<LedgerIndex>>, D::Error> {
    <LedgerIndex as serde::Deserialize>::deserialize(deserializer)
        .map(|index| Arc::new(Mutex::new(index)))
}

/// The plan cache as a snapshot holds it: the cache itself, not the lock
/// the two copies of the views share it through (sprawling-SPEC.md 8-99).
pub(super) fn encode_plans<S: serde::Serializer>(
    plans: &Arc<Mutex<PlanView>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&*PlanView::take_back(plans), serializer)
}

/// The plan cache `encode_plans` wrote, behind a lock of its own.
pub(super) fn decode_plans<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Arc<Mutex<PlanView>>, D::Error> {
    <PlanView as serde::Deserialize>::deserialize(deserializer)
        .map(|plans| Arc::new(Mutex::new(plans)))
}

impl SnapshotFold for Views {
    const DIR: &'static str = "views";

    fn fold_version() -> u32 {
        views_fold_version()
    }

    fn empty(city_root: &Path) -> Views {
        Views::new(city_root)
    }

    fn decode(city_root: &Path, bytes: &[u8]) -> Result<Views, AxError> {
        Views::decode(city_root, bytes)
    }

    fn encode(&self) -> Result<Vec<u8>, AxError> {
        Views::encode(self)
    }

    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.apply(record)
    }

    /// The views answer later questions through the index, and name the
    /// history by the epoch read from its genesis line.
    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError> {
        self.hold_index(index, ledger_dir)
    }

    /// The index the snapshot carried is brought up to the ledger: only
    /// the bytes appended since the cut are read, and a segment that
    /// shrank or vanished rebuilds it (storage-SPEC 8-4).
    fn resumed(&mut self, ledger_dir: &Path) -> Result<(), AxError> {
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        index
            .refresh(ledger_dir)
            .map(|_| ())
            .map_err(storage::StorageError::into_ax)
    }
}

impl Views {
    /// The folded state, without the fields `Views::new` rebuilds.
    ///
    /// # Errors
    /// `StorageFatal` when a field refuses to serialise.
    pub fn encode(&self) -> Result<Vec<u8>, AxError> {
        postcard::to_allocvec(self).map_err(|fault| {
            AxError::failure(AxCode::StorageFatal, "encode the views", fault.to_string())
                .with_recovery(
                    "restart the server; the views fold from the ledger without a snapshot",
                )
        })
    }

    /// A second copy of these views, folded to the same record and sharing
    /// their ledger index, plan cache, vault and halt, for the fold thread
    /// to alternate with (sprawling-SPEC.md 8-99): a clone, which copies
    /// every folded field and shares what sits behind an `Arc`, so the
    /// copy starts where this one stands without the history being read
    /// again (accounting-SPEC.md 8-19).
    ///
    /// # Errors
    /// None today; the `Result` stays until its caller in
    /// `bin::assembly::listening` drops the `?` (sprawling-SPEC.md 8-144).
    pub fn twin(&self) -> Result<Views, AxError> {
        Ok(self.clone())
    }

    /// Cuts a snapshot of these views at `record`, the last record they
    /// folded. Its `canonical_line` is the ledger's line, so the cut reads
    /// nothing back from the ledger (sprawling-SPEC.md 8-91).
    ///
    /// # Errors
    /// `InvalidArgs` when the record does not serialise, and those of
    /// [`start::cut_at`].
    pub fn cut_snapshot_at(&self, record: &EventRecord) -> Result<(), AxError> {
        let last = (record.seq(), record.canonical_line()?);
        start::cut_at(
            &kernel::layout::CityLayout::new(&self.city_root).ledger(),
            self,
            Some(&last),
        )
    }

    /// The views `encode` wrote, served from `city_root`.
    ///
    /// # Errors
    /// `CasCorrupt` when the bytes are not an encoding of this build's
    /// `Views`; the caller folds from genesis instead.
    pub fn decode(city_root: &Path, bytes: &[u8]) -> Result<Views, AxError> {
        let folded: Views = postcard::from_bytes(bytes).map_err(|fault| {
            AxError::failure(AxCode::CasCorrupt, "decode the views", fault.to_string())
                .with_recovery("none needed: the views fold from genesis and a new snapshot is cut")
        })?;
        let fresh = Views::new(city_root);
        Ok(Views {
            city_root: fresh.city_root,
            machine: fresh.machine,
            vault: fresh.vault,
            ..folded
        })
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
