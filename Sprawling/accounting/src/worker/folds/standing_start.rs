// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash, EventRecord};
use storage::LedgerIndex;

use super::collaboration::CollaborationFold;
use super::{Entrance, Governance, SessionOrigins, Standing};
use crate::views::snapshot::start::SnapshotFold;

const STANDING_FOLD_RULES: &str = "standing-fold-e2ee39bedd7b050b";

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct StandingFolds {
    book: gateway::EndpointBook,
    governance: Governance,
    collaboration: CollaborationFold,
    entrance: Entrance,
    origins: SessionOrigins,
}

impl StandingFolds {

    pub(super) fn settle(self, cut: Result<(), AxError>) -> Result<Standing, AxError> {
        Ok(Standing {
            book: self.book,
            governance: self.governance,
            collaboration: self.collaboration.settle()?,
            entrance: self.entrance,
            origins: self.origins,
            cut,
        })
    }
}

impl SnapshotFold for StandingFolds {
    const DIR: &'static str = "standing";

    fn fold_version() -> u32 {
        let rules = [env!("CARGO_PKG_VERSION"), STANDING_FOLD_RULES].join("\n");
        let [a, b, c, d, ..] = *B3Hash::digest(rules.as_bytes()).as_bytes();
        u32::from_le_bytes([a, b, c, d])
    }

    fn empty(_city_root: &Path) -> StandingFolds {
        StandingFolds {
            book: gateway::EndpointBook::new(),
            governance: Governance::empty(),
            collaboration: CollaborationFold::default(),
            entrance: Entrance::default(),
            origins: SessionOrigins::default(),
        }
    }

    fn decode(_city_root: &Path, bytes: &[u8]) -> Result<StandingFolds, AxError> {
        postcard::from_bytes(bytes).map_err(|fault| {
            AxError::failure(AxCode::CasCorrupt, "decode the standing", fault.to_string())
                .with_recovery(
                    "none needed: the standing folds from genesis and a new snapshot is cut",
                )
        })
    }

    fn encode(&self) -> Result<Vec<u8>, AxError> {
        postcard::to_allocvec(self).map_err(|fault| {
            AxError::failure(
                AxCode::StorageFatal,
                "encode the standing",
                fault.to_string(),
            )
            .with_recovery(
                "restart the server; the standing folds from the ledger without a snapshot",
            )
        })
    }

    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.book.apply(record)?;
        self.governance
            .absorb(record.kind(), record.run(), record.addr(), record.data())?;
        self.collaboration.absorb(record)?;
        self.entrance.absorb(record.data());
        self.origins
            .absorb(record.kind(), record.run(), record.addr(), record.data())
    }

    fn keep_index(&mut self, _index: LedgerIndex, _ledger_dir: &Path) -> Result<(), AxError> {
        Ok(())
    }

    fn resumed(&mut self, _ledger_dir: &Path) -> Result<(), AxError> {
        Ok(())
    }
}

pub(in crate::worker) fn as_json_text<T: serde::Serialize, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&serde_json::to_string(value).map_err(serde::ser::Error::custom)?)
}

pub(in crate::worker) fn from_json_text<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: serde::de::DeserializeOwned,
    D: serde::Deserializer<'de>,
{
    let text = <String as serde::Deserialize>::deserialize(deserializer)?;
    serde_json::from_str(&text).map_err(serde::de::Error::custom)
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
