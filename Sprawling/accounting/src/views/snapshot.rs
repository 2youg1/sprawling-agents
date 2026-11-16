// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use std::sync::{Arc, Mutex};

use crate::plan_view::PlanView;
use kernel::{AxCode, AxError, B3Hash, EventRecord};
use storage::LedgerIndex;

use super::Views;

pub mod start;

use start::SnapshotFold;

const VIEWS_FOLD_RULES: &str = "views-fold-0376b6783767b594";

pub(crate) fn views_fold_version() -> u32 {
    let rules = [env!("CARGO_PKG_VERSION"), VIEWS_FOLD_RULES].join("\n");
    let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
    u32::from_le_bytes([a, b, c, d])
}

pub(super) fn fresh_index() -> std::sync::Arc<std::sync::Mutex<storage::LedgerIndex>> {
    std::sync::Arc::new(std::sync::Mutex::new(storage::LedgerIndex::empty()))
}

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

pub(super) fn decode_index<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Arc<Mutex<LedgerIndex>>, D::Error> {
    <LedgerIndex as serde::Deserialize>::deserialize(deserializer)
        .map(|index| Arc::new(Mutex::new(index)))
}

pub(super) fn encode_plans<S: serde::Serializer>(
    plans: &Arc<Mutex<PlanView>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&*PlanView::take_back(plans), serializer)
}

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

    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError> {
        self.hold_index(index, ledger_dir)
    }

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

    pub fn encode(&self) -> Result<Vec<u8>, AxError> {
        postcard::to_allocvec(self).map_err(|fault| {
            AxError::failure(AxCode::StorageFatal, "encode the views", fault.to_string())
                .with_recovery(
                    "restart the server; the views fold from the ledger without a snapshot",
                )
        })
    }

    pub fn twin(&self) -> Result<Views, AxError> {
        Ok(self.clone())
    }

    pub fn cut_snapshot_at(&self, record: &EventRecord) -> Result<(), AxError> {
        let last = (record.seq(), record.canonical_line()?);
        start::cut_at(
            &kernel::layout::CityLayout::new(&self.city_root).ledger(),
            self,
            Some(&last),
        )
    }

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
pub(crate) mod tests;
