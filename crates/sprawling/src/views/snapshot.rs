// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The views as the bytes a snapshot holds (sprawling-SPEC 8-91).

use std::path::Path;

use std::sync::{Arc, Mutex};

use accounting::plan_view::PlanView;
use kernel::{AxCode, AxError, B3Hash, EventRecord, Seq};
use memory::LedgerIndex;

use super::Views;

pub(crate) mod start;

use start::SnapshotFold;

/// Changed whenever a fold rule or the encoding of `Views` changes within
/// one version of this binary; a new version discards every snapshot
/// anyway, because `views_fold_version` hashes the version in with this.
/// The suffix is the digest of a fixed fixture's encoding, which the
/// tests beside this file hold, so the encoding cannot move alone.
const VIEWS_FOLD_RULES: &str = "views-fold-a41c5ca5671d61ce";

/// The `fold_version` a views snapshot is cut and accepted under.
pub(crate) fn views_fold_version() -> u32 {
    let rules = [env!("CARGO_PKG_VERSION"), VIEWS_FOLD_RULES].join("\n");
    let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// The index a new or decoded `Views` starts with: empty, refreshed by
/// the first question that reads the ledger.
pub(super) fn fresh_index() -> std::sync::Arc<std::sync::Mutex<memory::LedgerIndex>> {
    std::sync::Arc::new(std::sync::Mutex::new(memory::LedgerIndex::empty()))
}

/// The plan cache as a snapshot holds it: the cache itself, not the lock
/// the two copies of the views share it through (sprawling-SPEC.md 8-93).
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
    /// history by the epoch read from its genesis line. After a resume
    /// the index starts empty and is refreshed by the first question
    /// that reads the ledger.
    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError> {
        self.hold_index(index, ledger_dir)
    }
}

impl Views {
    /// The folded state, without the fields `Views::new` rebuilds.
    ///
    /// # Errors
    /// `StorageFatal` when a field refuses to serialise.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, AxError> {
        postcard::to_allocvec(self).map_err(|fault| {
            AxError::failure(AxCode::StorageFatal, "encode the views", fault.to_string())
                .with_recovery(
                    "restart the server; the views fold from the ledger without a snapshot",
                )
        })
    }

    /// A second copy of these views, folded to the same record and sharing
    /// their ledger index and plan cache, for the fold thread to alternate
    /// with (sprawling-SPEC.md 8-93). Made through the encoding a snapshot
    /// holds, which carries every folded field, so the copy starts where
    /// this one stands without the history being read again.
    ///
    /// # Errors
    /// Those of [`Views::encode`] and [`Views::decode`].
    pub(crate) fn twin(&self) -> Result<Views, AxError> {
        let copy = Views::decode(&self.city_root, &self.encode()?)?;
        Ok(Views {
            index: Arc::clone(&self.index),
            plans: Arc::clone(&self.plans),
            machine: self.machine.clone(),
            vault: self.vault.clone(),
            registry: self.registry,
            ..copy
        })
    }

    /// The last line these views folded, read through their index, which
    /// is where a snapshot of them is cut. `None` before genesis is
    /// folded.
    ///
    /// # Errors
    /// `StorageFatal` when the index lock is poisoned, and the ledger
    /// read's own failure.
    pub(crate) fn last_folded_line(
        &self,
        ledger_dir: &Path,
    ) -> Result<Option<(Seq, Vec<u8>)>, AxError> {
        let Some(seq) = self.next_unfolded.value().checked_sub(1).map(Seq::new) else {
            return Ok(None);
        };
        let index = self.index.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the last folded line",
                "the ledger index lock is poisoned",
            )
            .with_recovery("restart the server; the index is rebuilt from the ledger")
        })?;
        index
            .reader(ledger_dir)
            .line_at(seq)
            .map(|line| Some((seq, line)))
            .map_err(memory::MemoryError::into_ax)
    }

    /// The views `encode` wrote, served from `city_root`.
    ///
    /// # Errors
    /// `CasCorrupt` when the bytes are not an encoding of this build's
    /// `Views`; the caller folds from genesis instead.
    pub(crate) fn decode(city_root: &Path, bytes: &[u8]) -> Result<Views, AxError> {
        let folded: Views = postcard::from_bytes(bytes).map_err(|fault| {
            AxError::failure(AxCode::CasCorrupt, "decode the views", fault.to_string())
                .with_recovery("none needed: the views fold from genesis and a new snapshot is cut")
        })?;
        let fresh = Views::new(city_root);
        Ok(Views {
            city_root: fresh.city_root,
            index: fresh.index,
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
